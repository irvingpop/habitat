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

mod handlers;

use depot;
use http_gateway::app::prelude::*;
use hab_net::oauth::github::GitHubClient;
use hab_core::event::EventLogger;
use http_gateway;
use iron;
use mount::Mount;
use persistent::{self, Read};
use staticfile::Static;

use self::handlers::*;
use config::Config;

struct ApiSrv;
impl HttpGateway for ApiSrv {
    const APP_NAME: &'static str = "builder-api";

    type Config = Config;

    fn add_middleware(config: Arc<Self::Config>, chain: &mut iron::Chain) {
        chain.link(persistent::Read::<GitHubCli>::both(
            GitHubClient::new(&*config),
        ));
        chain.link(Read::<EventLog>::both(
            EventLogger::new(&config.log_dir, config.events_enabled),
        ));
        chain.link_after(Cors);
    }

    fn mount(config: Arc<Self::Config>, chain: iron::Chain) -> Mount {
        let depot = depot::DepotUtil::new(config.depot.clone());
        let depot_chain = depot::server::router(depot).unwrap();
        let mut mount = Mount::new();
        if let Some(ref path) = config.ui.root {
            debug!("Mounting UI at filepath {}", path);
            mount.mount("/", Static::new(path));
        }
        mount.mount("/v1", chain);
        mount.mount("/v1/depot", depot_chain);
        mount
    }

    fn router(config: Arc<Self::Config>) -> Router {
        let basic = Authenticated::new(&*config);
        router!(
            status: get "/status" => status,
            authenticate: get "/authenticate/:code" => github_authenticate,

            jobs: post "/jobs" => XHandler::new(job_create).before(basic.clone()),
            job: get "/jobs/:id" => job_show,
            job_log: get "/jobs/:id/log" => job_log,
            job_group_promote: post "/jobs/group/:id/promote/:channel" => {
                XHandler::new(job_group_promote).before(basic.clone())
            },
            rdeps: get "/rdeps/:origin/:name" => rdeps_show,

            user_invitations: get "/user/invitations" => {
                XHandler::new(list_account_invitations).before(basic.clone())
            },
            user_origins: get "/user/origins" => {
                XHandler::new(list_user_origins).before(basic.clone())
            },

            // NOTE: Each of the handler functions for projects currently
            // short-circuits processing if trying to do anything with a
            // non-"core" origin, since we're not enabling Builder for any
            // other origins at the moment.
            projects: post "/projects" => XHandler::new(project_create).before(basic.clone()),
            project: get "/projects/:origin/:name" => project_show,
            project_jobs: get "/projects/:origin/:name/jobs" => project_jobs,
            edit_project: put "/projects/:origin/:name" => {
                XHandler::new(project_update).before(basic.clone())
            },
            delete_project: delete "/projects/:origin/:name" => {
                XHandler::new(project_delete).before(basic.clone())
            }
        )
    }
}

pub fn run(config: Config) -> AppResult<()> {
    http_gateway::start::<ApiSrv>(config)
}
