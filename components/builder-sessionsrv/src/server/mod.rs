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

mod handlers;

use std::sync::Arc;

use hab_net::app::prelude::*;
use hab_net::oauth::github::GitHubClient;
use protocol::sessionsrv::*;

use config::{Config, PermissionsCfg};
use data_store::DataStore;
use error::{Error, Result};

lazy_static! {
    static ref DISPATCH_TABLE: DispatchTable<SessionSrv> = {
        let mut map = DispatchTable::new();
        map.register(AccountGet::descriptor_static(None), handlers::account_get);
        map.register(AccountGetId::descriptor_static(None), handlers::account_get_id);
        map.register(SessionCreate::descriptor_static(None), handlers::session_create);
        map.register(SessionGet::descriptor_static(None), handlers::session_get);
        map.register(AccountInvitationListRequest::descriptor_static(None),
            handlers::account_invitation_list);
        map.register(AccountOriginInvitationCreate::descriptor_static(None),
            handlers::account_origin_invitation_create);
        map.register(AccountOriginInvitationAcceptRequest::descriptor_static(None),
            handlers::account_origin_invitation_accept
        );
        map.register(AccountOriginListRequest::descriptor_static(None),
            handlers::account_origin_list_request);
        map.register(AccountOriginCreate::descriptor_static(None), handlers::account_origin_create);
        map
    };
}

#[derive(Clone)]
pub struct ServerState {
    datastore: DataStore,
    github: Arc<Box<GitHubClient>>,
    permissions: Arc<PermissionsCfg>,
}

impl ServerState {
    pub fn new(cfg: &Config) -> Result<Self> {
        Ok(ServerState {
            datastore: DataStore::new(cfg)?,
            github: Arc::new(Box::new(GitHubClient::new(cfg))),
            permissions: Arc::new(cfg.permissions.clone()),
        })
    }
}

impl AppState for ServerState {
    fn is_initialized(&self) -> bool {
        true
    }
}

struct SessionSrv;
impl Dispatcher for SessionSrv {
    const APP_NAME: &'static str = "builder-sessionsrv";
    const PROTOCOL: Protocol = Protocol::SessionSrv;

    type Config = Config;
    type Error = Error;
    type State = ServerState;

    fn dispatch_table() -> &'static DispatchTable<Self> {
        &DISPATCH_TABLE
    }
}

pub fn run(config: Config) -> Result<()> {
    let state = ServerState::new(&config)?;
    // JW TODO: uncomment this ya dummy
    // state.datastore.setup()?;
    app_start::<SessionSrv>(config, state).map_err(Error::App)
}
