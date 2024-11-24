#!/bin/bash

git fetch
git rebase
cargo build -r
#sudo nice -n -19 ./target/release/clueless
#

sudo ./target/release/clueless

