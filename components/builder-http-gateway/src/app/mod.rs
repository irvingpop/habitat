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

pub mod prelude;

use std::sync::Arc;
use std::thread;

use core::os::process;
use hab_net;
use hab_net::app::error::AppResult;
use iron;
use iron::prelude::*;
use mount::Mount;
use router::Router;

use config::GatewayCfg;
use conn::RouteBroker;
use http::middleware::{Cors, XRouteClient};

pub trait HttpGateway {
    const APP_NAME: &'static str;

    type Config: GatewayCfg;

    fn add_middleware(Arc<Self::Config>, &mut iron::Chain) {
        ()
    }

    fn mount(Arc<Self::Config>, chain: iron::Chain) -> Mount {
        let mut mount = Mount::new();
        mount.mount("/", chain);
        mount
    }

    fn router(Arc<Self::Config>) -> Router;
}

/// Runs the main server and starts and manages all supporting threads. This function will
/// block the calling thread.
///
/// # Errors
///
/// * HTTP server could not start
pub fn start<T>(cfg: T::Config) -> AppResult<()>
where
    T: HttpGateway,
{
    let cfg = Arc::new(cfg);
    let mut chain = Chain::new(T::router(cfg.clone()));
    T::add_middleware(cfg.clone(), &mut chain);
    chain.link_before(XRouteClient);
    chain.link_after(Cors);
    let mount = T::mount(cfg.clone(), chain);
    let mut server = Iron::new(mount);
    server.threads = cfg.handler_count();
    let http_listen_addr = (cfg.listen_addr().clone(), cfg.listen_port());
    thread::Builder::new()
        .name("http-handler".to_string())
        .spawn(move || server.http(http_listen_addr))
        .unwrap();
    info!(
        "HTTP Gateway listening on {}:{}",
        cfg.listen_addr(),
        cfg.listen_port()
    );
    info!("{} is ready to go.", T::APP_NAME);
    RouteBroker::start(net_ident(), cfg.route_addrs())?;
    Ok(())
}

fn net_ident() -> String {
    let hostname = hab_net::hostname().unwrap();
    let pid = process::current_pid();
    format!("{}@{}", pid, hostname)
}
