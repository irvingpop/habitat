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

//! # Example Application
//!
//! ```
//! use std::process;
//! use net::app::prelude::*;
//!
//! pub mod config {
//!     use net::app::config::*;
//!
//!     #[derive(Default)]
//!     pub struct MyAppConfig {
//!         pub routers: Vec<RouterAddr>,
//!         pub shards: Vec<ShardId>,
//!         pub worker_threads: usize,
//!     }
//!
//!     impl AppConfig for MyAppConfig {
//!         fn route_addrs(&self) -> &[RouterAddr] {
//!             self.router_addrs.as_slice()
//!         }
//!
//!         fn shards(&self) -> Option<&[ShardId]> {
//!             Some(self.shards.as_slice())
//!         }
//!
//!         fn worker_count(&self) -> usize {
//!             self.worker_threads
//!         }
//!     }
//! }
//!
//! pub mod error {
//!     use std::error;
//!
//!     pub enum MyAppError {
//!         MomsSpaghetti
//!     }
//!
//!     impl error::Error for MyAppError {
//!         // ...
//!     }
//! }
//!
//! #[derive(Default)]
//! pub struct MyAppState;
//! impl AppState for MyAppState {
//!     fn is_iniitalized() -> bool {
//!         true
//!     }
//! }
//!
//! struct MyApp;
//! impl Dispatcher for MyApp {
//!     type Config = MyAppConfig;
//!     type Error = error::MyAppError;
//!     type State = MyAppState;
//!
//!     fn dispatch(
//!         request: &mut protocol::Message,
//!         conn: &mut RouteClient,
//!         state: &mut Self::State
//!     ) -> Result<(), Self::Error> {
//!         // handle message
//!     }
//! }
//!
//! fn main() {
//!     let config = MyAppConfig::default();
//!     let state = MyAppState::default();
//!     if let Err(err) = app::start<MyApp>::(config, state) {
//!         error!("{}", err);
//!         process::exit(1);
//!     }
//! }
//! ```

pub mod config;
pub mod error;
pub mod prelude;
mod dispatcher;

use std::collections::HashSet;

use core::os::process;
use protocol::{self, routesrv};
use uuid::Uuid;
use zmq::{self, Error as ZError};

use self::config::AppCfg;
use self::error::{AppError, AppResult};
use self::dispatcher::{Dispatcher, DispatcherPool};
use conn::{self, ConnErr};
use socket::{DEFAULT_CONTEXT, ToAddrString};

enum RecvResult {
    /// Signals to the main loop which sockets have pending messages to be processed. `.0` signals
    /// that the router socket has messages while `.1` signals the dispatcher queue.
    OnMessage((bool, bool, bool)),
    Shutdown,
    Timeout,
}

/// Apply to a struct containing worker state that will be passed as a mutable reference on each
/// call of `dispatch()` to an implementer of `Dispatcher`.
pub trait AppState: Clone + Send {
    fn is_initialized(&self) -> bool;
}

struct Application<T: Dispatcher> {
    /// Application configuration.
    pub config: T::Config,
    heartbeat: protocol::Message,
    /// Internal message buffer for proxying messages between router and dispatcher sockets.
    recv_buf: [zmq::Message; 2],
    registration: protocol::Message,
    pipe_in: zmq::Socket,
    pipe_out: zmq::Socket,
    router_sock: zmq::Socket,
    /// Set of RouteSrv's net identity.
    routers: HashSet<Vec<u8>>,
}

impl<T> Application<T>
where
    T: Dispatcher,
{
    fn new(config: T::Config) -> AppResult<Self> {
        let net_ident = net_ident();
        let router_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::ROUTER)?;
        router_sock.set_identity(net_ident.as_bytes())?;
        router_sock.set_probe_router(true)?;
        let pipe_out = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        let pipe_in = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        let mut registration = routesrv::Registration::new();
        registration.set_protocol(T::PROTOCOL);
        if let Some(ref shards) = config.shards() {
            registration.set_shards(shards.to_vec());
        }
        Ok(Application {
            config: config,
            heartbeat: protocol::Message::build(&routesrv::Heartbeat::new())?,
            recv_buf: [zmq::Message::new()?, zmq::Message::new()?],
            registration: protocol::Message::build(&registration)?,
            pipe_out: pipe_out,
            pipe_in: pipe_in,
            router_sock: router_sock,
            routers: HashSet::default(),
        })
    }

    fn run(mut self, state: T::State) -> AppResult<()> {
        for addr in self.config.route_addrs() {
            self.router_sock.connect(&addr.to_addr_string())?;
        }
        let pipe_in = format!("inproc://net.dispatcher.in.{}", Uuid::new_v4());
        let pipe_out = format!("inproc://net.dispatcher.out.{}", Uuid::new_v4());
        self.pipe_in.bind(&pipe_in)?;
        self.pipe_out.bind(&pipe_out)?;

        DispatcherPool::<T>::new(pipe_in, pipe_out, &self.config, state).run();
        info!("{} is ready to go.", T::APP_NAME);
        loop {
            trace!("waiting for message");
            // JW TODO: I should switch this to using the actual route client
            match self.wait_recv() {
                RecvResult::OnMessage((router, pipe_in, pipe_out)) => {
                    trace!(
                        "received messages, router={}, pipe-in={}, pipe-out={}",
                        router,
                        pipe_in,
                        pipe_out
                    );
                    // Handle completed work before new work
                    if pipe_in {
                        trace!("OnReply, dispatcher->router");
                        proxy_message(
                            &mut self.pipe_in,
                            &mut self.router_sock,
                            &mut self.recv_buf[0],
                        )?;
                    }
                    if pipe_out {
                        trace!("OnRequest, dispatcher->router");
                        // Remove dealer's blank frame
                        self.pipe_out.recv(&mut self.recv_buf[0], 0).map_err(
                            ConnErr::Socket,
                        )?;
                        trace!("{:?}", self.recv_buf[0]);
                        proxy_message(
                            &mut self.pipe_out,
                            &mut self.router_sock,
                            &mut self.recv_buf[0],
                        )?;
                    }
                    if router {
                        self.router_sock.recv(&mut self.recv_buf[0], 0).map_err(
                            ConnErr::Socket,
                        )?;
                        self.router_sock.recv(&mut self.recv_buf[1], 0).map_err(
                            ConnErr::Socket,
                        )?;
                        if !self.recv_buf[1].get_more() {
                            trace!("OnConnect, {:?}", self.recv_buf[0]);
                            conn::send_to(
                                &self.router_sock,
                                &mut self.registration,
                                &*self.recv_buf[0],
                            )?;
                            self.routers.insert(self.recv_buf[0].to_vec());
                            continue;
                        }
                        // JW TODO: we need to figure out if we received a message ourselves
                        // and not proxy it to the dispatchers if so.
                        trace!("OnMessage, router->dispatcher");
                        for msg in self.recv_buf.iter() {
                            trace!("proxy-message, {:?}", msg);
                            self.pipe_in.send(msg, zmq::SNDMORE).map_err(
                                ConnErr::Socket,
                            )?;
                        }
                        proxy_message(
                            &mut self.router_sock,
                            &mut self.pipe_in,
                            &mut self.recv_buf[1],
                        )?;
                    }
                }
                RecvResult::Timeout => {
                    trace!("recv timeout, sending heartbeat");
                    for addr in self.routers.iter() {
                        conn::send_to(&self.router_sock, &mut self.heartbeat, addr)?;
                    }
                }
                RecvResult::Shutdown => {
                    info!("received shutdown signal, shutting down...");
                    break;
                }
            }
        }
        Ok(())
    }

    fn wait_recv(&mut self) -> RecvResult {
        let mut items = [
            self.router_sock.as_poll_item(zmq::POLLIN),
            self.pipe_in.as_poll_item(zmq::POLLIN),
            self.pipe_out.as_poll_item(zmq::POLLIN),
        ];
        // JW TODO: switch to a tickless timer
        match zmq::poll(&mut items, 30_000) {
            Ok(count) if count < 0 => unreachable!("zmq::poll, returned with a negative count"),
            Ok(count) if count == 0 => return RecvResult::Timeout,
            Ok(count) => trace!("application received '{}' POLLIN events", count),
            Err(ZError::EAGAIN) => return RecvResult::Timeout,
            Err(ZError::EINTR) |
            Err(ZError::ETERM) => return RecvResult::Shutdown,
            Err(ZError::EFAULT) => panic!("zmq::poll, the provided _items_ was not valid (NULL)"),
            Err(err) => unreachable!("zmq::poll, returned an unexpected error, {:?}", err),
        }
        RecvResult::OnMessage((
            items[0].is_readable(),
            items[1].is_readable(),
            items[2].is_readable(),
        ))
    }
}

pub fn start<T>(cfg: T::Config, state: T::State) -> AppResult<()>
where
    T: Dispatcher,
{
    let app = Application::<T>::new(cfg)?;
    app.run(state)
}

fn net_ident() -> String {
    let hostname = super::hostname().unwrap();
    let pid = process::current_pid();
    format!("{}@{}", pid, hostname)
}

/// Proxy messages from one socket to another.
fn proxy_message(
    source: &mut zmq::Socket,
    destination: &mut zmq::Socket,
    buf: &mut zmq::Message,
) -> AppResult<()> {
    loop {
        if !buf.get_more() {
            break;
        }
        match source.recv(buf, 0) {
            Ok(()) => {
                trace!("proxy-message, {:?}", buf);
                let flags = if buf.get_more() { zmq::SNDMORE } else { 0 };
                destination.send(&*buf, flags).map_err(ConnErr::Socket)?;
            }
            Err(err) => return Err(AppError::from(ConnErr::Socket(err))),
        }
    }
    Ok(())
}
