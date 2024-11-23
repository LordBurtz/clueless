#!/bin/bash

git fetch
git rebase
cargo build -r
sudo ./target/release/clueless
