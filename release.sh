#!/bin/bash
cargo build --all --release
mkdir -p target/latest 2>/dev/null
mv target/release/client target/latest/client.darwin.amd64
mv target/release/daemon target/latest/daemon.darwin.amd64
