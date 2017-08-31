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

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate bitflags;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as core;
extern crate hyper;
extern crate hyper_openssl;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate persistent;
extern crate protobuf;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate time;
extern crate unicase;
extern crate uuid;
extern crate zmq;

pub mod app;
pub mod config;
pub mod conn;
pub mod error;
pub mod oauth;
pub mod privilege;
pub mod socket;

use std::process::Command;

pub use self::error::{ErrCode, NetError, NetOk, NetResult};

pub fn hostname() -> NetResult<String> {
    let output = match Command::new("sh")
        .arg("-c")
        .arg("hostname | awk '{printf \"%s\", $NF; exit}'")
        .output() {
        Ok(output) => output,
        Err(e) => {
            let err = NetError::new(ErrCode::SYS, "net:hostname:0");
            warn!("{}, {}", err, e);
            return Err(err);
        }
    };
    match output.status.success() {
        true => {
            debug!(
                "Hostname address is {}",
                String::from_utf8_lossy(&output.stdout)
            );
            match String::from_utf8(output.stdout) {
                Ok(hostname) => Ok(hostname),
                Err(err) => {
                    warn!("lookup host, {}", err);
                    Err(NetError::new(ErrCode::SYS, "net::hostname:1"))
                }
            }
        }
        false => {
            warn!(
                "Hostname address command returned: OUT: {} ERR: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            Err(NetError::new(ErrCode::SYS, "net:hostname:2"))
        }
    }
}
