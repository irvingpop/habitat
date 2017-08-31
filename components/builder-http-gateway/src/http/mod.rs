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

//! A module containing the HTTP server and handlers for servicing client requests

pub mod controller;
pub mod headers;
pub mod helpers;
pub mod middleware;
pub mod rendering;

use hab_net::ErrCode;
use iron::status::Status;

pub fn net_err_to_http(err: ErrCode) -> Status {
    match err {
        ErrCode::BUG => Status::InternalServerError,
        ErrCode::TIMEOUT => Status::GatewayTimeout,
        ErrCode::REMOTE_REJECTED => Status::NotAcceptable,
        ErrCode::BAD_REMOTE_REPLY => Status::BadGateway,
        ErrCode::ENTITY_NOT_FOUND => Status::NotFound,
        ErrCode::NO_SHARD => Status::ServiceUnavailable,
        ErrCode::ACCESS_DENIED => Status::Unauthorized,
        ErrCode::SESSION_EXPIRED => Status::Unauthorized,
        ErrCode::ENTITY_CONFLICT => Status::Conflict,
        ErrCode::SOCK => Status::ServiceUnavailable,
        ErrCode::DATA_STORE => Status::ServiceUnavailable,
        ErrCode::AUTH_SCOPE => Status::Forbidden,
        ErrCode::WORKSPACE_SETUP => Status::InternalServerError,
        ErrCode::SECRET_KEY_FETCH => Status::BadGateway,
        ErrCode::SECRET_KEY_IMPORT => Status::InternalServerError,
        ErrCode::VCS_CLONE => Status::BadGateway,
        ErrCode::BUILD => Status::InternalServerError,
        ErrCode::POST_PROCESSOR => Status::InternalServerError,
        ErrCode::REG_CONFLICT => Status::InternalServerError,
        ErrCode::REMOTE_UNAVAILABLE => Status::ServiceUnavailable,
        ErrCode::SYS => Status::InternalServerError,
        ErrCode::GROUP_NOT_COMPLETE => Status::Forbidden,
        ErrCode::PARTIAL_JOB_GROUP_PROMOTE => Status::Forbidden,
    }
}
