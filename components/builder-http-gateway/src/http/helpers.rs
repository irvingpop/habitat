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

use std::collections::HashMap;
use std::str::FromStr;

use bldr_core::data_structures::PartialJobGroupPromote;
use core::channel::{STABLE_CHANNEL, UNSTABLE_CHANNEL};
use hab_net::{ErrCode, NetError, NetOk, NetResult};
use iron::prelude::*;
use protocol::originsrv::{CheckOriginAccessRequest, CheckOriginAccessResponse, Origin,
                          OriginChannel, OriginChannelCreate, OriginChannelGet, OriginGet,
                          OriginPackage, OriginPackageChannelListRequest,
                          OriginPackageChannelListResponse, OriginPackageGet,
                          OriginPackageGroupPromote, OriginPackageIdent,
                          OriginPackagePlatformListRequest, OriginPackagePlatformListResponse,
                          OriginPackagePromote};
use protocol::scheduler::{Group, GroupGet, GroupState, Project, ProjectState};
use protocol::sessionsrv::{Session, SessionCreate, SessionGet};

use super::controller::route_message;

// Get channels for a package
pub fn channels_for_package_ident(
    req: &mut Request,
    package: &OriginPackageIdent,
) -> Option<Vec<String>> {
    let mut opclr = OriginPackageChannelListRequest::new();
    opclr.set_ident(package.clone());

    match route_message::<OriginPackageChannelListRequest, OriginPackageChannelListResponse>(
        req,
        &opclr,
    ) {
        Ok(channels) => {
            let list: Vec<String> = channels
                .get_channels()
                .iter()
                .map(|channel| channel.get_name().to_string())
                .collect();

            Some(list)
        }
        Err(_) => None,
    }
}

// Get platforms for a package
pub fn platforms_for_package_ident(
    req: &mut Request,
    package: &OriginPackageIdent,
) -> Option<Vec<String>> {
    let mut opplr = OriginPackagePlatformListRequest::new();
    opplr.set_ident(package.clone());

    match route_message::<OriginPackagePlatformListRequest, OriginPackagePlatformListResponse>(
        req,
        &opplr,
    ) {
        Ok(p) => Some(p.get_platforms().to_vec()),
        Err(_) => None,
    }
}

pub fn get_origin<T>(req: &mut Request, origin: T) -> NetResult<Origin>
where
    T: ToString,
{
    let mut request = OriginGet::new();
    request.set_name(origin.to_string());
    route_message::<OriginGet, Origin>(req, &request)
}

pub fn check_origin_access<T>(req: &mut Request, account_id: u64, origin: T) -> NetResult<bool>
where
    T: ToString,
{
    let mut request = CheckOriginAccessRequest::new();
    request.set_account_id(account_id);
    request.set_origin_name(origin.to_string());
    match route_message::<CheckOriginAccessRequest, CheckOriginAccessResponse>(req, &request) {
        Ok(response) => Ok(response.get_has_access()),
        Err(err) => Err(err),
    }
}

pub fn create_channel(
    req: &mut Request,
    origin: &str,
    channel: &str,
    session_id: u64,
) -> NetResult<OriginChannel> {
    let mut origin = get_origin(req, origin)?;
    let mut request = OriginChannelCreate::new();
    request.set_owner_id(session_id);
    request.set_origin_name(origin.take_name());
    request.set_origin_id(origin.get_id());
    request.set_name(channel.to_string());
    route_message::<OriginChannelCreate, OriginChannel>(req, &request)
}

pub fn promote_package_to_channel(
    req: &mut Request,
    ident: &OriginPackageIdent,
    channel: &str,
    session_id: u64,
) -> NetResult<NetOk> {
    if !check_origin_access(req, session_id, ident.get_origin())? {
        return Err(NetError::new(
            ErrCode::ACCESS_DENIED,
            "core:promote-package-to-channel:0",
        ));
    }
    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel.to_string());

    let origin_channel = route_message::<OriginChannelGet, OriginChannel>(req, &channel_req)?;
    let mut request = OriginPackageGet::new();
    request.set_ident(ident.clone());
    let package = route_message::<OriginPackageGet, OriginPackage>(req, &request)?;
    let mut promote = OriginPackagePromote::new();
    promote.set_channel_id(origin_channel.get_id());
    promote.set_package_id(package.get_id());
    promote.set_ident(ident.clone());
    route_message::<OriginPackagePromote, NetOk>(req, &promote)
}

pub fn promote_job_group_to_channel(
    req: &mut Request,
    group_id: u64,
    channel: &str,
    session_id: u64,
) -> NetResult<NetOk> {
    let mut group_get = GroupGet::new();
    group_get.set_group_id(group_id);
    let group = route_message::<GroupGet, Group>(req, &group_get)?;

    // This only makes sense if the group is complete. If the group isn't complete, return now and
    // let the user know. Check the completion state by checking the individual project states,
    // as if this is called by the scheduler it needs to promote the group before marking it
    // Complete.
    if group.get_projects().iter().any(|&ref p| {
        p.get_state() == ProjectState::NotStarted || p.get_state() == ProjectState::InProgress
    })
    {
        return Err(NetError::new(
            ErrCode::GROUP_NOT_COMPLETE,
            "hg:promote-job-group:0",
        ));
    }

    let mut failed_projects = Vec::new();
    let mut origin_map = HashMap::new();

    // JW TODO: refactor
    // let mut channel_fn = |org, chn, ses| {
    //     if chn == STABLE_CHANNEL || chn == UNSTABLE_CHANNEL {
    //         return Ok(());
    //     }
    //     match create_channel(req, org, chn, ses) {
    //         Ok(_) => Ok(()),
    //         Err(err) => {
    //             if err.code() == ErrCode::ENTITY_CONFLICT {
    //                 Ok(())
    //             } else {
    //                 Err(err)
    //             }
    //         }
    //     }
    // };

    // We can't assume that every project in the group belongs to the same origin. It's entirely
    // possible that there are multiple origins present within the group. Because of this, there's
    // no way to atomically commit the entire promotion at once. It's possible origin shards can be
    // on different machines, so for now, the best we can do is partition the projects by origin,
    // and commit each origin at once. Ultimately, it'd be nice to have a way to atomically commit
    // the entire promotion at once, but that would require a cross-shard tool that we don't
    // currently have.
    for project in group.get_projects().into_iter() {
        if project.get_state() == ProjectState::Success {
            let ident = OriginPackageIdent::from_str(project.get_ident()).unwrap();

            let mut project_list = origin_map.entry(ident.get_origin().to_string()).or_insert(
                Vec::new(),
            );
            project_list.push(project);
        } else {
            failed_projects.push(project.get_ident().to_string());
        }
    }

    // Now that we've sorted all the projects into a HashMap keyed on origin, we can process them,
    // one origin at a time. We do "core" first, since it's not possible to do the entire operation
    // atomically. Instead, we prioritize the core origin since it's the most important one.
    if let Some(core_projects) = origin_map.remove("core") {
        // JW TODO: refactor
        // if let Err(e) = channel_fn("core", channel, session_id) {
        //     return Err(e);
        // }

        let promote_result = do_group_promotion(req, channel, core_projects, "core", session_id);
        if promote_result.is_err() {
            return promote_result;
        }
    }

    for (origin, projects) in origin_map.iter() {
        // JW TODO: refactor
        // if let Err(e) = channel_fn(&origin, channel, session_id) {
        //     return Err(e);
        // }

        let promote_result =
            do_group_promotion(req, channel, projects.to_vec(), &origin, session_id);
        if promote_result.is_err() {
            return promote_result;
        }
    }

    if failed_projects.is_empty() {
        Ok(NetOk::new())
    } else {
        let pjgp = PartialJobGroupPromote {
            group_id: group.get_id(),
            failed_projects: failed_projects,
        };
        Err(NetError::new(
            ErrCode::PARTIAL_JOB_GROUP_PROMOTE,
            "hg:promote-job-group:1",
        ))
    }
}

pub fn authenticate_with_auth_token(req: &mut Request, auth_token: &str) -> NetResult<u64> {
    let mut request = SessionGet::new();
    request.set_token(auth_token.to_string());

    match route_message::<SessionGet, Session>(req, &request) {
        Ok(session) => Ok(session.get_id()),
        Err(err) => {
            if err.code() == ErrCode::SESSION_EXPIRED {
                let mut create = SessionCreate::new();
                create.set_token(auth_token.to_string());

                let session = route_message::<SessionCreate, Session>(req, &create)?;
                Ok(session.get_id())
            } else {
                Err(err)
            }
        }
    }
}

fn do_group_promotion(
    req: &mut Request,
    channel: &str,
    projects: Vec<&Project>,
    origin: &str,
    session_id: u64,
) -> NetResult<NetOk> {
    let mut ocg = OriginChannelGet::new();
    ocg.set_origin_name(origin.to_string());
    ocg.set_name(channel.to_string());

    let mut package_ids = Vec::new();
    let channel = route_message::<OriginChannelGet, OriginChannel>(req, &ocg)?;
    if !check_origin_access(req, session_id, origin)? {
        return Err(NetError::new(
            ErrCode::ACCESS_DENIED,
            "core:group-promote:0",
        ));
    }

    for project in projects {
        let opi = OriginPackageIdent::from_str(project.get_ident()).unwrap();
        let mut opg = OriginPackageGet::new();
        opg.set_ident(opi);
        let op = route_message::<OriginPackageGet, OriginPackage>(req, &opg)?;
        package_ids.push(op.get_id());
    }

    let mut opgp = OriginPackageGroupPromote::new();
    opgp.set_channel_id(channel.get_id());
    opgp.set_package_ids(package_ids);
    opgp.set_origin(origin.to_string());

    route_message::<OriginPackageGroupPromote, NetOk>(req, &opgp)
}
