#!/usr/bin/env bash
set -x
set -eo pipefail

bash scripts/init_db.sh
bash scripts/init_local_redis.sh