#!/bin/bash

export PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile)

mkdir -p /hab/svc/builder-api
cat <<EOT > /hab/svc/builder-api/user.toml
[github]
url = "https://api.github.com"
web_url = "https://github.com"
client_id = "0c2f738a7d0bd300de10"
client_secret = "438223113eeb6e7edf2d2f91a232b72de72b9bdf"

[web]
app_url          = "http://localhost/v1"
community_url    = "https://www.habitat.sh/community"
docs_url         = "https://www.habitat.sh/docs"
environment      = "production"
friends_only     = false
source_code_url  = "https://github.com/habitat-sh/habitat"
tutorials_url    = "https://www.habitat.sh/tutorials"
www_url          = "https://www.habitat.sh"

[depot]
builds_enabled = true
EOT

mkdir -p /hab/svc/builder-jobsrv
cat <<EOT > /hab/svc/builder-jobsrv/user.toml
[datastore]
password = "$PGPASSWORD"

[archive]
backend = "local"
EOT

mkdir -p /hab/svc/builder-originsrv
cat <<EOT > /hab/svc/builder-originsrv/user.toml
[datastore]
password = "$PGPASSWORD"
EOT

mkdir -p /hab/svc/builder-scheduler
cat <<EOT > /hab/svc/builder-scheduler/user.toml
auth_token = "8e2c9a90675e0c11af0cacf86ae404cc883335c3"
depot_url = "http://localhost:9636/v1/depot"

[datastore]
password = "$PGPASSWORD"
EOT

mkdir -p /hab/svc/builder-sessionsrv
cat <<EOT > /hab/svc/builder-sessionsrv/user.toml
[datastore]
password = "$PGPASSWORD"

[permissions]
admin_team = 1995301
build_worker_teams = [1995301]
early_access_teams = [1995301]

[github]
url = "https://api.github.com"
client_id = "0c2f738a7d0bd300de10"
client_secret = "438223113eeb6e7edf2d2f91a232b72de72b9bdf"
EOT

mkdir -p /hab/svc/builder-worker
cat <<EOT > /hab/svc/builder-worker/user.toml
auth_token = "8e2c9a90675e0c11af0cacf86ae404cc883335c3"
depot_url = "http://localhost:9636/v1/depot"
auto_publish = true
EOT
