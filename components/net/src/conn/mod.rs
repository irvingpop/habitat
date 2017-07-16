// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

//! Contains types and functions for sending and receiving messages to and from a message broker
//! connected to one or more `RouteSrv`. All messages are routed through a `RouteSrv` and forwarded
//! to the appropriate receiver of a message.

mod error;

use protobuf;
use protocol::Routable;
use protocol::message::{Header, Message, RouteInfo, Txn};
use zmq::{self, Error as ZError};

pub use self::error::ConnErr;
use error::{ErrCode, NetError, NetResult};
use socket::DEFAULT_CONTEXT;

/// Time to wait before timing out a message receive for a `RouteConn`.
pub const RECV_TIMEOUT_MS: i32 = 5_000;
/// Time to wait before timing out a message send for a `RouteBroker` to a router.
<<<<<<< Updated upstream
const SEND_TIMEOUT_MS: i32 = 5_000;
// ZeroMQ address for the application's RouteBroker's queue.
const ROUTE_INPROC_ADDR: &'static str = "inproc://route-broker";

/// A messaging RouteBroker for proxying messages from clients to one or more `RouteSrv` and vice
/// versa.
pub struct RouteBroker {
    client_sock: zmq::Socket,
    router_sock: zmq::Socket,
}

impl RouteBroker {
    /// Create a new `RouteBroker`
    ///
    /// # Errors
    ///
    /// * A socket cannot be created within the given `zmq::Context`
    /// * A socket cannot be configured
    ///
    /// # Panics
    ///
    /// * Could not read `zmq::Context` due to deadlock or poisoning
    fn new(net_ident: String) -> Result<Self, ConnErr> {
        let fe = (**DEFAULT_CONTEXT).as_mut().socket(zmq::ROUTER)?;
        let be = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER)?;
        fe.set_identity(net_ident.as_bytes())?;
        be.set_rcvtimeo(RECV_TIMEOUT_MS)?;
        be.set_sndtimeo(SEND_TIMEOUT_MS)?;
        be.set_immediate(true)?;
        Ok(RouteBroker {
            client_sock: fe,
            router_sock: be,
        })
    }

    /// Helper function for creating a new `RouteClient`.
    ///
    /// # Errors
    ///
    /// * Could not connect to `RouteBroker`
    /// * Could not create socket
    ///
    /// # Panics
    ///
    /// * Could not read `zmq::Context` due to deadlock or poisoning
    pub fn connect() -> Result<RouteClient, ConnErr> {
        let conn = RouteClient::new()?;
        conn.connect(ROUTE_INPROC_ADDR)?;
        Ok(conn)
    }

    /// Create a new `RouteBroker` and run it in a separate thread. This function will block the
    /// calling thread until the new broker has successfully started.
    ///
    /// # Panics
    ///
    /// * RouteBroker crashed during startup
    pub fn run(net_ident: String, routers: &Vec<RouterAddr>) -> JoinHandle<()> {
        let (tx, rx) = mpsc::sync_channel(1);
        let addrs = routers.iter().map(|a| a.to_addr_string()).collect();
        let handle = thread::Builder::new()
            .name("router-broker".to_string())
            .spawn(move || {
                let mut broker = Self::new(net_ident).unwrap();
                broker.start(tx, addrs).unwrap();
            })
            .unwrap();
        match rx.recv() {
            Ok(()) => handle,
            Err(e) => panic!("router-broker thread startup error, err={}", e),
        }
    }

    // Main loop for `RouteBroker`.
    //
    // Binds front-end socket to ZeroMQ inproc address and connects to all routers. Sends a message
    // back to the caller over the given rendezvous channel to signal when ready.
    fn start(&mut self, rz: mpsc::SyncSender<()>, routers: Vec<String>) -> Result<(), ConnErr> {
        self.client_sock.bind(ROUTE_INPROC_ADDR)?;
        for addr in routers {
            self.router_sock.connect(&addr)?;
        }
        rz.send(()).unwrap();
        zmq::proxy(&mut self.client_sock, &mut self.router_sock)?;
        Ok(())
    }
}
=======
pub const SEND_TIMEOUT_MS: i32 = 5_000;
>>>>>>> Stashed changes

/// Message events signaling activity on the server listener.
pub enum ConnEvent {
    /// Occurs when a new connection was made to the listener.
    OnConnect,
    /// Occurs when the listener receives a new, well formed, message.
    OnMessage,
}

/// Client connection for sending and receiving messages to and from the service cluster through
/// a running `RouteBroker`.
pub struct RouteClient {
    /// Connection for replying to requests from a RouteSrv.
    rep_conn: RouteConn,
    /// Connection for making requests to a RouteSrv.
    req_conn: RouteConn,
    /// Internal message buffer for receiving and sending messages.
    msg_buf: Message,
}

impl RouteClient {
    /// Create a new `RouteClient`
    ///
    /// # Errors
    ///
    /// * Socket(s) could not be created
    pub fn new() -> Result<Self, ConnErr> {
        let rep_socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::REP)?;
        let req_socket = (**DEFAULT_CONTEXT).as_mut().socket(zmq::REQ)?;
        Ok(RouteClient {
            rep_conn: RouteConn::new(rep_socket)?,
            req_conn: RouteConn::new(req_socket)?,
            msg_buf: Message::default(),
        })
    }

    fn wait_recv(
        socket: &zmq::Socket,
        msg: &mut Message,
        buf: &mut zmq::Message,
        timeout: i64,
    ) -> Result<(), ConnErr> {
        match poll_socket(socket, zmq::POLLIN, timeout) {
            Ok(count) => {
                trace!("route-client, received '{}' POLLIN events", count);
                read_header(socket, msg, buf)?;
                if msg.header().has_route_info() {
                    read_route_info(socket, msg, buf)?;
                }
                if msg.header().has_txn() {
                    read_txn(socket, msg, buf)?;
                }
                read_body(socket, msg, buf)?;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Connect the `RouteClient` to the given endpoint.
    pub fn connect<T, U>(&self, reply_queue: T, request_queue: U) -> Result<(), ConnErr>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.rep_conn.connect(&reply_queue)?;
        self.req_conn.connect(&request_queue)?;
        Ok(())
    }

    /// Routes a message to the connected broker, through a router, and to appropriate service,
    /// waits for a response, and then returns the response.
    pub fn route<M, T>(&mut self, msg: &M) -> NetResult<T>
    where
        M: Routable,
        T: protobuf::MessageStatic,
    {
        self.route_async(msg)?;
        match self.wait_recv_out(-1) {
            Ok(()) => {
                if self.msg_buf.message_id() == NetError::message_id() {
                    match NetError::parse(&self.msg_buf) {
                        Ok(err) => return Err(err),
                        Err(err) => error!("{}", err),
                    }
                }
                match self.msg_buf.parse::<T>() {
                    Ok(reply) => Ok(reply),
                    Err(e) => {
                        let err = NetError::new(ErrCode::BUG, "net:client-route:2");
                        error!("{}, {}", err, e);
                        Err(err)
                    }
                }
            }
            Err(e @ ConnErr::Timeout) => {
                let err = NetError::new(ErrCode::TIMEOUT, "net:client-route:3");
                warn!("{}, {}", err, e);
                return Err(err);
            }
            Err(e) => {
                // JW TODO: We can do a lot better here by turning ConnErr into a NetErr.
                let err = NetError::new(ErrCode::BUG, "net:client-route:4");
                error!("{}, {}", err, e);
                return Err(err);
            }
        }
    }

    /// Asynchronously routes a message to the connected broker, through a router, and to
    /// appropriate service.
    pub fn route_async<T>(&mut self, msg: &T) -> NetResult<()>
    where
        T: Routable + protobuf::MessageStatic,
    {
        // JW TODO: handle this unwrap
        self.msg_buf.populate(msg).unwrap();
        if let Err(e) = self.req_conn.route(&self.msg_buf) {
            let err = NetError::new(ErrCode::SOCK, "net:client-route:1");
            error!("{}, {}", err, e);
            return Err(err);
        }
        Ok(())
    }

    /// Send a reply to a transactional message.
    pub fn route_reply<T>(&self, message: &mut Message, reply: &T) -> NetResult<()>
    where
        T: protobuf::Message,
    {
        if let Err(e) = self.rep_conn.route_reply(message, reply) {
            let err = NetError::new(ErrCode::SOCK, "net:client-reply:1");
            error!("{}, {}", err, e);
            return Err(err);
        }
        Ok(())
    }

    pub fn wait_recv_in(&mut self, msg: &mut Message, timeout: i64) -> Result<(), ConnErr> {
        Self::wait_recv(
            &self.rep_conn.socket,
            msg,
            &mut self.req_conn.msg_buf,
            timeout,
        )
    }

    pub fn wait_recv_out(&mut self, timeout: i64) -> Result<(), ConnErr> {
        Self::wait_recv(
            &self.req_conn.socket,
            &mut self.msg_buf,
            &mut self.req_conn.msg_buf,
            timeout,
        )
    }
}

/// Underlying connection struct for sending and receiving messages to and from a RouteSrv.
pub struct RouteConn {
    /// Internal message buffer for receiving message parts.
    msg_buf: zmq::Message,
    /// Listening socket for all connections into the Router.
    socket: zmq::Socket,
}

impl RouteConn {
    pub fn new(socket: zmq::Socket) -> Result<Self, ConnErr> {
        socket.set_rcvtimeo(RECV_TIMEOUT_MS)?;
        socket.set_sndtimeo(SEND_TIMEOUT_MS)?;
        socket.set_immediate(true)?;
        Ok(RouteConn {
            msg_buf: zmq::Message::new()?,
            socket: socket,
        })
    }

    pub fn bind<T>(&self, endpoint: T) -> Result<(), ConnErr>
    where
        T: AsRef<str>,
    {
        Ok(self.socket.bind(endpoint.as_ref())?)
    }

    pub fn connect<T>(&self, endpoint: T) -> Result<(), ConnErr>
    where
        T: AsRef<str>,
    {
        self.socket.connect(endpoint.as_ref())?;
        Ok(())
    }

    pub fn route(&self, message: &Message) -> Result<(), ConnErr> {
        route(&self.socket, message)
    }

    pub fn route_reply<T>(&self, message: &mut Message, reply: &T) -> Result<(), ConnErr>
    where
        T: protobuf::Message,
    {
        route_reply(&self.socket, message, reply)
    }

    /// Attempts to wait for a value on this receiver, returning an error if the corresponding
    /// connection has shutdown, or if it waits more than timeout.
    ///
    /// This function will always block the current thread if there is no data available and. Once
    /// a message is sent to the corresponding connection, the thread will wake up write the
    /// the contents into `message`.
    pub fn wait_recv(&mut self, message: &mut Message, timeout: i64) -> Result<ConnEvent, ConnErr> {
        match poll_socket(&self.socket, zmq::POLLIN, timeout) {
            Ok(count) => {
                trace!("route-conn, received '{}' POLLIN events", count);
                match read_into(&self.socket, message, &mut self.msg_buf) {
                    Ok(event) => Ok(event),
                    Err(err) => {
                        if let Err(err) = read_until_end(&self.socket, &mut self.msg_buf) {
                            error!("error while reading to end of message, {}", err)
                        }
                        Err(err)
                    }
                }
            }
            Err(err) => Err(err),
        }
    }
}

/// Send a message directly to the given receiver ignoring all identities in the given message.
pub fn route(socket: &zmq::Socket, message: &Message) -> Result<(), ConnErr> {
    for identity in message.identities.iter() {
        socket.send(identity, zmq::SNDMORE)?;
        trace!("route: {}", String::from_utf8_lossy(identity));
    }
    socket.send(&[], zmq::SNDMORE)?;
    trace!("route: {}", message);
    send_header(socket, message)?;
    if message.header().has_route_info() {
        send_route_info(socket, message)?;
    }
    if message.header().has_txn() {
        send_txn(socket, message)?;
    }
    send_body(socket, message)
}

pub fn route_reply<T>(socket: &zmq::Socket, message: &mut Message, reply: &T) -> Result<(), ConnErr>
where
    T: protobuf::Message,
{
    message.populate_reply(reply)?;
    // route(socket, message)
    socket.send(&[], zmq::SNDMORE)?;
    trace!("route: {}", message);
    send_header(socket, message)?;
    if message.header().has_route_info() {
        send_route_info(socket, message)?;
    }
    if message.header().has_txn() {
        send_txn(socket, message)?;
    }
    send_body(socket, message)
}

pub fn send_to(socket: &zmq::Socket, message: &mut Message, dest: &[u8]) -> Result<(), ConnErr> {
    socket.send(dest, zmq::SNDMORE)?;
    socket.send(&[], zmq::SNDMORE)?;
    send_header(socket, message)?;
    if message.header().has_route_info() {
        send_route_info(socket, message)?;
    }
    if message.header().has_txn() {
        send_txn(socket, message)?;
    }
    send_body(socket, message)
}

fn poll_socket(
    socket: &zmq::Socket,
    events: zmq::PollEvents,
    timeout: i64,
) -> Result<u32, ConnErr> {
    match socket.poll(events, timeout) {
        Ok(count) if count < 0 => unreachable!("zmq::poll, returned a negative count"),
        Ok(count) if count == 0 => Err(ConnErr::Timeout),
        Ok(count) => Ok(count as u32),
        Err(ZError::EAGAIN) => Err(ConnErr::Timeout),
        Err(e @ ZError::EINTR) |
        Err(e @ ZError::ETERM) => Err(ConnErr::Shutdown(e)),
        Err(ZError::EFAULT) => unreachable!("zmq::poll, the provided _items_ was not valid (NULL)"),
        Err(err) => unreachable!("zmq::poll, returned an unexpected error, {:?}", err),
    }
}

fn read_into(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<ConnEvent, ConnErr> {
    read_identity(socket, message, buf)?;
    if !buf.get_more() {
        return Ok(ConnEvent::OnConnect);
    }
    read_header(socket, message, buf)?;
    if message.header().has_route_info() {
        if !buf.get_more() {
            return Err(ConnErr::NoRouteInfo);
        }
        read_route_info(socket, message, buf)?;
    }
    if message.header().has_txn() {
        if !buf.get_more() {
            return Err(ConnErr::NoTxn);
        }
        read_txn(socket, message, buf)?;
    }
    if !buf.get_more() {
        return Err(ConnErr::NoBody);
    }
    read_body(socket, message, buf)?;
    if buf.get_more() {
        warn!("received message with additional message parts");
        if let Err(err) = read_until_end(socket, buf) {
            error!("error while reading to end of message, {}", err)
        }
    }
    Ok(ConnEvent::OnMessage)
}

fn read_identity(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<(), ConnErr> {
    let mut first = true;
    loop {
        socket.recv(buf, 0)?;
        trace!("recv: identity, {}", String::from_utf8_lossy(buf));
        if buf.len() == 0 && first {
            return Err(ConnErr::NoIdentity);
        }
        if buf.len() == 0 {
            break;
        }
        message.identities.push(buf.to_vec());
        first = false;
    }
    Ok(())
}

fn read_header(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<(), ConnErr> {
    loop {
        socket.recv(buf, 0)?;
        if buf.len() == 0 {
            continue;
        }
        let header = Header::from_bytes(&*buf).map_err(ConnErr::BadHeader)?;
        trace!("recv: header, {:?}", header);
        message.set_header(header);
        break;
    }
    Ok(())
}

fn read_route_info(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<(), ConnErr> {
    socket.recv(buf, 0)?;
    let route_info = RouteInfo::from_bytes(&*buf).map_err(ConnErr::BadRouteInfo)?;
    trace!("recv: route-info, {}", route_info);
    message.set_route_info(route_info);
    Ok(())
}

fn read_txn(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<(), ConnErr> {
    socket.recv(buf, 0)?;
    let txn = Txn::from_bytes(&*buf).map_err(ConnErr::BadTxn)?;
    trace!("recv: txn, {}", txn);
    message.set_txn(txn);
    Ok(())
}

fn read_body(
    socket: &zmq::Socket,
    message: &mut Message,
    buf: &mut zmq::Message,
) -> Result<(), ConnErr> {
    socket.recv(buf, 0)?;
    trace!("recv: body, {:?}", buf);
    message.body = buf.to_vec();
    Ok(())
}

fn read_until_end(socket: &zmq::Socket, buf: &mut zmq::Message) -> Result<(), ConnErr> {
    loop {
        if !buf.get_more() {
            break;
        }
        socket.recv(buf, 0)?;
        trace!("recv: overflow, {:?}", buf);
    }
    Ok(())
}

fn send_body(socket: &zmq::Socket, message: &Message) -> Result<(), ConnErr> {
    socket.send(&*message.body, 0)?;
    Ok(())
}

fn send_header(socket: &zmq::Socket, message: &Message) -> Result<(), ConnErr> {
    let bytes = message.header().to_bytes()?;
    socket.send(&bytes, zmq::SNDMORE)?;
    Ok(())
}

fn send_route_info(socket: &zmq::Socket, message: &Message) -> Result<(), ConnErr> {
    let bytes = message.route_info().as_ref().unwrap().to_bytes()?;
    socket.send(&bytes, zmq::SNDMORE)?;
    Ok(())
}

fn send_txn(socket: &zmq::Socket, message: &Message) -> Result<(), ConnErr> {
    if let Some(txn) = message.txn() {
        let bytes = txn.to_bytes()?;
        socket.send(&bytes, zmq::SNDMORE)?;
    }
    Ok(())
}
