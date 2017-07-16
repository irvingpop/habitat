// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

use hab_net::{ErrCode, NetError};
use protocol::{net, routesrv};
use protocol::message::Message;

use super::ServerMap;
use conn::Conn;
use error::Result;

pub fn on_disconnect(_: &Conn, message: &mut Message, servers: &mut ServerMap) -> Result<()> {
    servers.drop(
        &message.route_info().unwrap().protocol(),
        message.sender().unwrap(),
    );
    Ok(())
}

pub fn on_heartbeat(_: &Conn, message: &mut Message, servers: &mut ServerMap) -> Result<()> {
    servers.renew(message.sender().unwrap());
    Ok(())
}

pub fn on_registration(conn: &Conn, message: &mut Message, servers: &mut ServerMap) -> Result<()> {
    let mut body = message.parse::<routesrv::Registration>()?;
    let protocol = body.get_protocol();
    let shards = body.take_shards();
    if servers.add(protocol, message.sender().unwrap().to_vec(), shards) {
        // JW TODO: we need to reply but we can't until the app/dispatcher uses a proper req socket
        // conn.route_reply(message, &net::NetOk::new())?;
    } else {
        let err = NetError::new(ErrCode::REG_CONFLICT, "rt:connect:1");
        warn!("{}", err);
        conn.route_reply(message, &*err)?;
    }
    Ok(())
}
