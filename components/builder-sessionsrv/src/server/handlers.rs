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

use std::env;

use hab_net::app::prelude::*;
use hab_net::{ErrCode, NetError};
use hab_net::privilege;

use protocol::net;
use protocol::sessionsrv as proto;

use super::ServerState;
use error::Result;

pub fn account_get_id(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountGetId>()?;
    match state.datastore.get_account_by_id(&req) {
        Ok(Some(account)) => conn.route_reply(msg, &account)?,
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "ss:account-get-id:0");
            conn.route_reply(msg, &*err)?;
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-get-id:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_get(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountGet>()?;
    match state.datastore.get_account(&req) {
        Ok(Some(account)) => conn.route_reply(msg, &account)?,
        Ok(None) => {
            let err = NetError::new(ErrCode::ENTITY_NOT_FOUND, "ss:account-get:0");
            conn.route_reply(msg, &*err)?;
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account-get:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn session_create(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let mut req = msg.parse::<proto::SessionCreate>()?;
    let mut is_admin = false;
    let mut is_early_access = false;
    let mut is_build_worker = false;

    if env::var_os("HAB_FUNC_TEST").is_some() {
        is_admin = true;
        is_early_access = true;
        is_build_worker = true;
    } else {
        let teams = match state.github.teams(req.get_token()) {
            Ok(teams) => teams,
            Err(_) => {
                let err = NetError::new(ErrCode::ACCESS_DENIED, "ss:session-create:0");
                conn.route_reply(msg, &*err)?;
                return Ok(());
            }
        };
        for team in teams {
            if team.id != 0 && team.id == state.permissions.admin_team {
                debug!(
                    "Granting feature flag={:?} for team={:?}",
                    privilege::ADMIN,
                    team.name
                );
                is_admin = true;
            }
            if team.id != 0 && state.permissions.early_access_teams.contains(&team.id) {
                debug!(
                    "Granting feature flag={:?} for team={:?}",
                    privilege::EARLY_ACCESS,
                    team.name
                );
                is_early_access = true;
            }
            if team.id != 0 && state.permissions.build_worker_teams.contains(&team.id) {
                debug!(
                    "Granting feature flag={:?} for team={:?}",
                    privilege::BUILD_WORKER,
                    team.name
                );
                is_build_worker = true;
            }
        }
    }

    // If only a token was filled in, let's grab the rest of the data from GH. We check email in
    // this case because although email is an optional field in the protobuf message, email is
    // required for access to builder.
    if req.get_email().is_empty() {
        match state.github.user(req.get_token()) {
            Ok(user) => {
                // Select primary email. If no primary email can be found, use any email. If
                // no email is associated with account return an access denied error.
                let email = match state.github.emails(req.get_token()) {
                    Ok(ref emails) => {
                        emails
                            .iter()
                            .find(|e| e.primary)
                            .unwrap_or(&emails[0])
                            .email
                            .clone()
                    }
                    Err(_) => {
                        let err = NetError::new(ErrCode::ACCESS_DENIED, "ss:session-create:2");
                        conn.route_reply(msg, &*err)?;
                        return Ok(());
                    }
                };

                req.set_extern_id(user.id);
                req.set_email(email);
                req.set_name(user.login);
                req.set_provider(proto::OAuthProvider::GitHub);
            }
            Err(_) => {
                let err = NetError::new(ErrCode::ACCESS_DENIED, "ss:session-create:3");
                conn.route_reply(msg, &*err)?;
                return Ok(());
            }
        }
    }

    match state.datastore.find_or_create_account_via_session(
        &req,
        is_admin,
        is_early_access,
        is_build_worker,
    ) {
        Ok(session) => conn.route_reply(msg, &session)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:session-create:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn session_get(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::SessionGet>()?;
    match state.datastore.get_session(&req) {
        Ok(Some(session)) => conn.route_reply(msg, &session)?,
        Ok(None) => {
            let err = NetError::new(ErrCode::SESSION_EXPIRED, "ss:auth:4");
            conn.route_reply(msg, &*err)?;
        }
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:auth:5");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_invitation_create(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountOriginInvitationCreate>()?;
    match state.datastore.create_account_origin_invitation(&req) {
        Ok(()) => conn.route_reply(msg, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account_origin_invitation_create:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_invitation_accept(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountOriginInvitationAcceptRequest>()?;
    match state.datastore.accept_origin_invitation(&req) {
        Ok(()) => conn.route_reply(msg, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account_origin_invitation_accept:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_create(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountOriginCreate>()?;
    match state.datastore.create_origin(&req) {
        Ok(()) => conn.route_reply(msg, &net::NetOk::new())?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account_origin_create:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_origin_list_request(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountOriginListRequest>()?;
    match state.datastore.get_origins_by_account(&req) {
        Ok(reply) => conn.route_reply(msg, &reply)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account_origin_list_request:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}

pub fn account_invitation_list(
    msg: &mut Message,
    conn: &mut RouteClient,
    state: &mut ServerState,
) -> Result<()> {
    let req = msg.parse::<proto::AccountInvitationListRequest>()?;
    match state.datastore.list_invitations(&req) {
        Ok(response) => conn.route_reply(msg, &response)?,
        Err(e) => {
            let err = NetError::new(ErrCode::DATA_STORE, "ss:account_invitation_list:1");
            error!("{}, {}", e, err);
            conn.route_reply(msg, &*err)?;
        }
    }
    Ok(())
}
