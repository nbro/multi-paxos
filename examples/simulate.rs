//! An example which simulates Paxos locally (on one machine).
//!
//! Run this example as follows
//!     RUST_LOG=multi_paxos=info cargo run --example simulate

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate multi_paxos;
extern crate serde;

use std::fmt::Debug;
use std::marker::Send;
use std::sync::{Arc, Barrier};
use std::thread;

use serde::de::DeserializeOwned;
use serde::Serialize;

use multi_paxos::configurations::get_config;
use multi_paxos::multi_paxos::{Acceptor, Client, Learner, Proposer};
use multi_paxos::multi_paxos::Runnable;

fn main() {
    env_logger::init();

    // Try all of these Copy and thread-safe values.
    // let value: bool = true;
    let value: u32 = 7;
    // let value: f32 = 3.14;
    // let value: usize = 10;
    // let value = 'a';
    // let value = (7, 3.14, true, 'a');

    // Copy is not implemented for String
    // let value = String::from("hello"); // error
    // let value = "hello"; // Lifetime problems: https://github.com/serde-rs/json/issues/331

    simulate(value);
}

fn simulate<T>(value: T)
    where T: Serialize + DeserializeOwned + Copy + Clone + Debug + Send + 'static + PartialEq,
{
    let config = get_config("Config");
    info!("Configurations = {:?}\n", config);

    let (num_of_clients, clients_address) = config["clients"];
    let (num_of_proposers, proposers_address) = config["proposers"];
    let (num_of_acceptors, acceptors_address) = config["acceptors"];
    let (num_of_learners, learners_address) = config["learners"];

    // Store all threads in vector so that they can be joined later.
    let mut all_threads = Vec::new();

    // To coordinate the execution of the threads. In particular, we want to send messages only when
    // all sockets have been created.
    let barrier = Arc::new(Barrier::new(
        num_of_clients + num_of_proposers + num_of_acceptors + num_of_learners,
    ));

    let mut uid: usize = 0;

    for _ in 0..num_of_clients {
        let c = barrier.clone();
        let client_thread: thread::JoinHandle<_> = thread::spawn(move || {
            let client = Client::new(uid, clients_address, proposers_address);
            c.wait();
            client.request(value);
        });

        all_threads.push(client_thread);
        uid += 1;
    }

    for _ in 0..num_of_proposers {
        let c = barrier.clone();
        let proposer_thread: thread::JoinHandle<_> = thread::spawn(move || {
            let mut proposer = Proposer::<T>::new(
                uid,
                proposers_address,
                acceptors_address,
                learners_address,
                num_of_acceptors,
            );
            c.wait();
            proposer.run();
        });
        all_threads.push(proposer_thread);
        uid += 1;
    }

    for _ in 0..num_of_acceptors {
        let c = barrier.clone();
        let acceptor_thread: thread::JoinHandle<_> = thread::spawn(move || {
            let mut acceptor = Acceptor::<T>::new(uid, acceptors_address, proposers_address);
            c.wait();
            acceptor.run();
        });

        all_threads.push(acceptor_thread);
        uid += 1;
    }

    for _ in 0..num_of_learners {
        let c = barrier.clone();
        let learner_thread: thread::JoinHandle<_> = thread::spawn(move || {
            let mut learner = Learner::<T>::new(uid, learners_address, proposers_address);
            c.wait();
            learner.run();
        });
        all_threads.push(learner_thread);
        uid += 1;
    }

    info!("Number of threads created = {:?}\n", all_threads.len());

    for thread_handle in all_threads {
        thread_handle.join().expect("Failed to join the child thread");
    }
}
