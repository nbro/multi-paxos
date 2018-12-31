#!/bin/bash

#RUST_LOG=multi_paxos=info cargo run --example start_proposer -- $1 $2
cargo run --example start_proposer -- "$1" "$2"