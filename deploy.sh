#!/bin/bash

git fetch
git rebase
RUSTFLAGS="-C target-feature=+aes,+sse2 -C target-cpu=native" cargo build -r
#sudo nice -n -19 ./target/release/clueless
#

echo "Running with ulimit $(ulimit -n)"
sudo ./target/release/clueless

