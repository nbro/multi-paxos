//! A script used to start one client, which infinitely reads from the standard input or, if more
//! than two command-line arguments are passed, it uses the 3rd, 4th, etc., arguments as the
//! proposal values.
//!
//! You can run this example as follows
//!     RUST_LOG=multi_paxos=info cargo run --example start_client -- <client_uid> Config
//! If you want to run this client interactively (i.e. provide one proposal at a time), or
//!     RUST_LOG=multi_paxos=info cargo run --example start_client -- <client_uid> Config p1 p2 ...
//! where p1, p2, etc., are the proposal numbers.

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate multi_paxos;
extern crate serde;
#[macro_use]
extern crate text_io;

use std::env;
use std::io;
use std::io::prelude::*;

use multi_paxos::configurations::get_config;
use multi_paxos::multi_paxos::Client;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    info!("{:?}", args);

    match args.len() {
        len if len >= 3 => {
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

            let (_, clients_address) = config["clients"];
            let (_, proposers_address) = config["proposers"];

            let client = Client::new(uid, clients_address, proposers_address);

            if len == 3 {
                loop {
                    print!("Enter the proposal: ");
                    io::stdout().flush().ok().expect("Could not flush stdout"); // print! is not very clever.
                    let value: usize = read!();
                    client.request(value);
                }
            } else {
                for proposal in args.iter().skip(3) {
                    let p: usize = match proposal.parse() {
                        Ok(n) => n,
                        Err(_) => {
                            eprintln!("Only proposals of type usize are for now supported.");
                            return;
                        }
                    };
                    client.request(p);
                }
            }
        }
        _ => {
            panic!("Expected 2 arguments (excluding file name)");
        }
    }
}
