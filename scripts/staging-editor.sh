#!/usr/bin/env bash
git add . 
cargo fmt --all 
git add . 
cargo fix --allow-staged
git add .
cargo clippy
git add .
cargo sqlx prepare -- --lib
git add .
