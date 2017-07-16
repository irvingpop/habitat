// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::error;
use std::marker::PhantomData;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use protobuf;
use protocol::{Message, Protocol};

use super::AppState;
use super::config::AppCfg;
use conn::{ConnErr, RouteClient};

/// Dispatchers connect to Message Queue Servers
pub trait Dispatcher: Sized + Send + 'static {
    const APP_NAME: &'static str;
    const PROTOCOL: Protocol;

    type Config: AppCfg;
    type Error: error::Error;
    type State: AppState;

    fn dispatch_table() -> &'static DispatchTable<Self>;

    /// Callback to perform dispatcher initialization.
    ///
    /// The default implementation will take your initial state and convert it into the actual
    /// state of the worker. Override this function if you need to perform additional steps to
    /// initialize your worker state.
    #[allow(unused_mut)]
    fn init(mut state: Self::State) -> Self::State {
        state
    }
}

pub trait Handler<T: Dispatcher>: Send + Sync + 'static {
    fn handle(&self, &mut Message, &mut RouteClient, &mut T::State) -> Result<(), T::Error>;
}

impl<T, F> Handler<T> for F
where
    T: Dispatcher,
    F: Sync
        + Send
        + 'static
        + Fn(&mut Message, &mut RouteClient, &mut T::State) -> Result<(), T::Error>,
{
    fn handle(
        &self,
        message: &mut Message,
        conn: &mut RouteClient,
        state: &mut T::State,
    ) -> Result<(), T::Error> {
        (*self)(message, conn, state)
    }
}

impl<T> Handler<T> for Box<Handler<T>>
where
    T: Dispatcher,
{
    fn handle(
        &self,
        message: &mut Message,
        conn: &mut RouteClient,
        state: &mut T::State,
    ) -> Result<(), T::Error> {
        (**self).handle(message, conn, state)
    }
}

pub struct DispatcherPool<T: Dispatcher> {
    state: T::State,
    reply_queue: Arc<String>,
    request_queue: Arc<String>,
    worker_count: usize,
    workers: Vec<mpsc::Receiver<()>>,
    marker: PhantomData<T>,
}

pub struct DispatchTable<T>(HashMap<&'static str, Box<Handler<T>>>);

impl<T> DispatchTable<T>
where
    T: Dispatcher,
{
    pub fn new() -> Self {
        DispatchTable(HashMap::new())
    }

    pub fn get(&self, message_id: &str) -> Option<&Box<Handler<T>>> {
        self.0.get(message_id)
    }

    pub fn register<H>(&mut self, msg: &'static protobuf::reflect::MessageDescriptor, handler: H)
    where
        H: Handler<T>,
    {
        if self.0.insert(msg.name(), Box::new(handler)).is_some() {
            panic!(
                "Attempted to register a second handler for message, '{}'",
                msg.name()
            );
        }
    }
}

impl<T> DispatcherPool<T>
where
    T: Dispatcher,
{
    pub fn new<C>(reply_queue: String, request_queue: String, config: &C, state: T::State) -> Self
    where
        C: AppCfg,
    {
        DispatcherPool {
            reply_queue: Arc::new(reply_queue),
            request_queue: Arc::new(request_queue),
            state: state,
            worker_count: config.worker_count(),
            workers: Vec::with_capacity(config.worker_count()),
            marker: PhantomData,
        }
    }

    /// Start a pool of message dispatchers.
    pub fn run(mut self) {
        for worker_id in 0..self.worker_count {
            self.spawn_dispatcher(worker_id);
        }
        thread::spawn(move || loop {
            for i in 0..self.worker_count {
                // Refactor this if/when the standard library ever stabilizes select for mpsc
                // https://doc.rust-lang.org/std/sync/mpsc/struct.Select.html
                match self.workers[i].try_recv() {
                    Err(mpsc::TryRecvError::Disconnected) => {
                        info!("Worker[{}] restarting...", i);
                        self.spawn_dispatcher(i);
                    }
                    Ok(msg) => warn!("Worker[{}] sent unexpected msg: {:?}", i, msg),
                    Err(mpsc::TryRecvError::Empty) => continue,
                }
            }
            thread::sleep(Duration::from_millis(500));
        });
    }

    fn spawn_dispatcher(&mut self, worker_id: usize) {
        let (tx, rx) = mpsc::sync_channel(1);
        let state = T::init(self.state.clone());
        let reply_queue = self.reply_queue.clone();
        let request_queue = self.request_queue.clone();
        thread::spawn(move || {
            worker_run::<T>(tx, worker_id, reply_queue, request_queue, state)
        });
        if rx.recv().is_ok() {
            debug!("worker[{}] ready", worker_id);
            self.workers.insert(worker_id, rx);
        } else {
            error!("worker[{}] failed to start", worker_id);
            self.workers.remove(worker_id);
        }
    }
}

fn dispatch<T>(message: &mut Message, conn: &mut RouteClient, state: &mut T::State)
where
    T: Dispatcher,
{
    trace!("dispatch, {}", message);
    match T::dispatch_table().get(message.message_id()) {
        Some(handler) => {
            if let Err(err) = (**handler).handle(message, conn, state) {
                error!("{}", err);
            }
        }
        None => {
            warn!("dispatch, recv unknown message, {}", message.message_id());
        }
    }
}

fn worker_run<T>(
    rz: mpsc::SyncSender<()>,
    id: usize,
    reply_queue: Arc<String>,
    request_queue: Arc<String>,
    mut state: T::State,
) where
    T: Dispatcher,
{
    debug_assert!(
        state.is_initialized(),
        "Dispatcher state not initialized! wrongfully \
        implements the `init()` callback or omits an override implementation where the default \
        implementation isn't enough to initialize the dispatcher's state?"
    );
    let mut message = Message::default();
    let mut conn = RouteClient::new().unwrap();
    conn.connect(&*reply_queue, &*request_queue).unwrap();
    rz.send(()).unwrap();
    loop {
        message.reset();
        trace!("worker[{}] waiting for message", id);
        match conn.wait_recv_in(&mut message, -1) {
            Ok(()) => (),
            Err(ConnErr::Shutdown(_)) => break,
            Err(err) => {
                warn!("worker[{}], {}", id, err);
                continue;
            }
        }
        dispatch::<T>(&mut message, &mut conn, &mut state);
    }
}
