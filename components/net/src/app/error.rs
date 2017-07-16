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

use std::collections::HashMap;
use std::error;
use std::fmt;
use std::io;

pub use protocol::net::{ErrCode, NetOk};
use hyper;
use protobuf;
use protocol;
use serde_json;
use zmq;

use conn;
use oauth;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Auth(oauth::github::AuthErr),
    Connection(conn::ConnErr),
    GitHubAPI(hyper::status::StatusCode, HashMap<String, String>),
    HttpClient(hyper::Error),
    HttpClientParse(hyper::error::ParseError),
    HttpResponse(hyper::status::StatusCode),
    IO(io::Error),
    Json(serde_json::Error),
    Protobuf(protobuf::ProtobufError),
    Protocol(protocol::ProtocolError),
    RequiredConfigField(&'static str),
    Sys,
    Zmq(zmq::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            AppError::Auth(ref e) => format!("GitHub Authentication error, {}", e),
            AppError::Connection(ref e) => format!("{}", e),
            AppError::GitHubAPI(ref c, ref m) => format!("[{}] {:?}", c, m),
            AppError::HttpClient(ref e) => format!("{}", e),
            AppError::HttpClientParse(ref e) => format!("{}", e),
            AppError::HttpResponse(ref e) => format!("{}", e),
            AppError::IO(ref e) => format!("{}", e),
            AppError::Json(ref e) => format!("{}", e),
            AppError::Protobuf(ref e) => format!("{}", e),
            AppError::Protocol(ref e) => format!("{}", e),
            AppError::RequiredConfigField(ref e) => {
                format!("Missing required field in configuration, {}", e)
            }
            AppError::Sys => format!("Internal system error"),
            AppError::Zmq(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for AppError {
    fn description(&self) -> &str {
        match *self {
            AppError::Auth(_) => "GitHub authorization error.",
            AppError::Connection(ref err) => err.description(),
            AppError::GitHubAPI(_, _) => "GitHub API error.",
            AppError::HttpClient(ref err) => err.description(),
            AppError::HttpClientParse(ref err) => err.description(),
            AppError::HttpResponse(_) => "Non-200 HTTP response.",
            AppError::IO(ref err) => err.description(),
            AppError::Json(ref err) => err.description(),
            AppError::Protobuf(ref err) => err.description(),
            AppError::Protocol(ref err) => err.description(),
            AppError::RequiredConfigField(_) => "Missing required field in configuration.",
            AppError::Sys => "Internal system error",
            AppError::Zmq(ref err) => err.description(),
        }
    }
}

impl From<conn::ConnErr> for AppError {
    fn from(err: conn::ConnErr) -> AppError {
        AppError::Connection(err)
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> AppError {
        AppError::IO(err)
    }
}

impl From<hyper::Error> for AppError {
    fn from(err: hyper::Error) -> AppError {
        AppError::HttpClient(err)
    }
}

impl From<oauth::github::AuthErr> for AppError {
    fn from(err: oauth::github::AuthErr) -> Self {
        AppError::Auth(err)
    }
}

impl From<protobuf::ProtobufError> for AppError {
    fn from(err: protobuf::ProtobufError) -> AppError {
        AppError::Protobuf(err)
    }
}

impl From<protocol::ProtocolError> for AppError {
    fn from(err: protocol::ProtocolError) -> AppError {
        AppError::Protocol(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> AppError {
        AppError::Json(err)
    }
}

impl From<zmq::Error> for AppError {
    fn from(err: zmq::Error) -> AppError {
        AppError::Zmq(err)
    }
}
