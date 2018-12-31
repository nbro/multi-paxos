//! A script used to start one acceptor, which will infinitely listen to incoming messages (from
//! proposers).
//!
//! You can run this example as follows
//!     RUST_LOG=multi_paxos=info cargo run --example start_acceptor -- <acceptor_uid> Config
//! where <acceptor_uid> is a non-negative number which should be unique (among all nodes).

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate multi_paxos;
extern crate serde;

use std::env;

use multi_paxos::configurations::get_config;
use multi_paxos::multi_paxos::Acceptor;
use multi_paxos::multi_paxos::Runnable;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);

    match args.len() {
        3 => {
            let uid = &args[1];
            let uid: usize = match uid.parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Error: second argument not an usize");
                    return;
                }
            };

            let config_file_name = &args[2];
            let config = get_config(config_file_name);

            let (_, proposers_address) = config["proposers"];
            let (_, acceptors_address) = config["acceptors"];

            let mut acceptor = Acceptor::<usize>::new(uid, acceptors_address, proposers_address);
            acceptor.run();
        }
        _ => {
            panic!("Expected 2 arguments (excluding file name)");
        }
    }
}
