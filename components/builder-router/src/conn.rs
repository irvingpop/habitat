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

use std::ops::{Deref, DerefMut};

pub use hab_net::conn::{ConnErr, ConnEvent};
use hab_net::conn::RouteConn;
use protocol::Message;
use zmq;

/// RouteSrv listener
pub struct Conn(RouteConn);

impl Conn {
    pub fn new(context: &mut zmq::Context) -> Result<Self, ConnErr> {
        let socket = context.socket(zmq::ROUTER)?;
        socket.set_router_mandatory(true)?;
        socket.set_probe_router(true)?;
        // JW TODO: Set an actual identity here
        socket.set_identity("WHATMAN".to_string().as_bytes())?;
        Ok(Conn(RouteConn::new(socket)?))
    }

    pub fn forward(&self, message: &mut Message, destination: Vec<u8>) -> Result<(), ConnErr> {
        if message.route_info().is_none() {
            return Err(ConnErr::NoRouteInfo);
        }
        message.identities.insert(0, destination);
        self.0.route(message)
    }
}

impl Deref for Conn {
    type Target = RouteConn;

    fn deref(&self) -> &RouteConn {
        &self.0
    }
}

impl DerefMut for Conn {
    fn deref_mut(&mut self) -> &mut RouteConn {
        &mut self.0
    }
}
