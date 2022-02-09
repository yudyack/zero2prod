#!/usr/bin/env bash
git add .
cargo fmt --all --check
git add .
cargo fix --allow-dirty
git add .
cargo sqlx prepare -- --lib
git add .
