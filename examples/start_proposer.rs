//! A script used to start one proposer, which will infinitely listen to incoming messages (either
//! from clients or acceptors.
//!
//! You can run this example as follows
//!     RUST_LOG=multi_paxos=info cargo run --example start_proposer -- <proposer_uid> Config
//! where <proposer_uid> is a non-negative number which should be unique (among all nodes).

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate multi_paxos;
extern crate serde;

use std::env;

use multi_paxos::configurations::get_config;
use multi_paxos::multi_paxos::Proposer;
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
            let (num_of_acceptors, acceptors_address) = config["acceptors"];
            let (_, learners_address) = config["learners"];

            let mut proposer = Proposer::<usize>::new(
                uid,
                proposers_address,
                acceptors_address,
                learners_address,
                num_of_acceptors,
            );
            proposer.run();
        }
        _ => {
            panic!("Expected 2 arguments (excluding file name)");
        }
    }
}
