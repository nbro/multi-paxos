#!/bin/bash

#RUST_LOG=multi_paxos=info cargo run --example start_client -- $1 $2
xargs cargo run --example start_client -- "$1" "$2"
