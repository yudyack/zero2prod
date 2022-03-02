#!/usr/bin/env bash
cargo watch -x check -x "test | bunyan" -x "run | bunyan"
