#!/bin/bash

#RUST_LOG=multi_paxos=info cargo run --example start_acceptor -- $1 $2
cargo run --example start_acceptor -- "$1" "$2"