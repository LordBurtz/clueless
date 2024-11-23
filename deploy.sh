#!/bin/bash

git pull
cargo build -r
sudo ./target/debug/clueless
