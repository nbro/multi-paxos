# Implementation of Atomic Broadcast using Multi-Paxos

## Introduction

In distributed computing, an [atomic broadcast](https://en.wikipedia.org/wiki/Atomic_broadcast) (AB) is a type of [broadcast](https://en.wikipedia.org/wiki/Broadcasting_(networking)) (that is, a method of sending a message to all recipients simultaneously) where all _correct_ processes (or, in general, nodes), among `n` processes (of which `f` are _faulty_), receive the same set of messages, in the same order, or they receive none of the messages (hence the adjective "atomic"). More specifically, if a process `p` atomically broadcast message `a`, `b` and `c`, then all `n - f` correct processes receive the messages `a`, `b` and `c`, in some order (e.g. `[b, a, c]`), which is equal to the order received by the other processes. A faulty process is a process that eventually crashes (i.e. stops and, in this case, does not recover). A correct process is a non-faulty process. 

Note that, in this setting, using just a simple broadcast, nodes may receive messages out-of-order (or, in general, asynchronously), because messages can be lost while being sent through the network or they can undergo arbitrarily delays (e.g. because of congestions in the network). Therefore, if a process `p` just sent the messages in the order `a`, `b` and `c`, it would not be guaranteed that the messages would be received in that order (i.e. `[a, b, c]`), by all other processes. Hence the need to implement more sophisticated algorithms to guarantee e.g. atomicity.

Given that AB and [consensus](https://en.wikipedia.org/wiki/Consensus_(computer_science)) are known to be "equivalent" problems (see [Unreliable failure detectors for reliable distributed systems](https://www.cs.utexas.edu/~lorenzo/corsi/cs380d/papers/p225-chandra.pdf) by Tushar Deepak Chandra and Sam Toueg, for more info), we can use an algorithm (or a modification of it) which solves consensus to implement AB (or vice-versa). Consensus is the problem of agreeing on one value (e.g. a number) among a group of participants (e.g. processes). A famous algorithm which solves consensus is called [Paxos](https://en.wikipedia.org/wiki/Paxos_(computer_science)). To agree on a set of values (instead of just one), we can run an "instance" of the consensus algorithm for each of the values in the set. A common algorithm to solve this problem of agreeing on multiple values ("multi-consensus") is called _Multi-Paxos_.

More specifically, we can implement AB using an algorithm (or a modification of it) which implements consensus as follows. Suppose that each process initially has the set of messages `M = {a, b, c}`. Given that this is a set, the order of the messages is not pre-determined. We want to agree on an order of the messages in `M`. More specifically, we want all correct processes to either receive (or decide) one (and only one!) of the following sequences `[a, b, c]`, `[a, c, b]`, `[b, a, c]`, `[b, c, a]`, `[c, a, b]`, `[c, b, a]`. To do that, we can start an "instance" of the consensus algorithm for each element in the sequence. For example, initially, process `p` may "propose" `a`, whereas process `q` may propose `b`. After one "instance" of the consensus algorithm, all processes have either decided `a` or (exclusive) `b`, say, it is `b`. At that point, all correct processes know that `b` is the first element of the sequence; so they also know that the order of `a` and `c` has not yet been decided. After that, another "instance" of the consensus algorithm is started. Without loss of generality, assume that, in this instance, the value decided is `a`, then the second element of the sequence is `a`. At this point, all correct processes know that the sequence of messages atomically broadcast so far is `[b, a]`. Given that there are only 3 messages, the processes actually know that the final sequence of atomically broadcast messages must be `[b, a, c]`, that is, they can just use `|M| - 1` instances of the basic consensus algorithm to atomically broadcast `|M|` messages. 

This is the idea behind "Multi-Paxos", which uses the basic Paxos algorithm to agree on one value.

## Implementation

This is a Rust implementation of Multi-Paxos, which is used to atomically broadcast a set of messages (as explained above). This implementation uses UDP sockets to exchange messages. It is based on [IP multicast](https://en.wikipedia.org/wiki/IP_multicast). So, "proposers" (of the Paxos algorithm) are associated with a multicast group (address). Similarly, clients, acceptors and learners are also associated with other multicast groups. Hence, there are 4 multicast groups (and thus 4 IP multicast addresses) involved: one for each role. See the configuration file [`Config.toml`](Config.toml), where these addresses are specified.

The naming conventions used follow the pseudo-code of the Paxos algorithm under the folder [`images/pseudocode`](./images/pseudocode). The images under the folder [`images`](./images) are screenshots of the slides by prof. [Fernando Pedone](https://www.inf.usi.ch/faculty/pedone/).

### Assumptions

This implementation of the Multi-Paxos algorithm has a few assumptions:

- There is only one quorum of acceptors
- All processes have only one role, so a process cannot be e.g. a proposer and acceptor at the same time.
- Processes fail and do not recover (i.e., they are "fail-stop"), so all state can be kept in RAM

## How to install Rust?

Before proceeding, you first need to install the Rust compiler, [`rustc`](https://doc.rust-lang.org/rustc/what-is-rustc.html), and the Rust package manager, [`cargo`](https://doc.rust-lang.org/cargo/) (if you don't have them installed yet). Please, follow the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) (which will tell you to actually install first `rustup`). If you have any problems during the installation of these tools, please, contact me.

## How to test this implementation?

Please, have a look at the [`README`](./tests/README.md) file under the `tests` folder (which is under this crate).

### How to run a client, acceptor, proposer and/or learner?

You can run as many clients, acceptors, proposers and/or learners as you need or wish. To do that, you can execute one of the following commands from the terminal. 

To run a client, execute

    RUST_LOG=multi_paxos=info xargs cargo run --example start_client -- <client_uid> Config < client_input.txt
    
See the file [`examples/start_client.rs`](./examples/start_client.rs) for more info.

Similarly, to run an acceptor, execute

    RUST_LOG=multi_paxos=info cargo run --example start_acceptor -- <acceptor_uid> Config

See the file [`examples/start_acceptor.rs`](./examples/start_acceptor.rs) for more info.

To run a proposer, execute

    RUST_LOG=multi_paxos=info cargo run --example start_proposer -- <proposer_uid> Config

See the file [`examples/start_proposer.rs`](./examples/start_proposer.rs) for more info.

Finally, to run a learner, execute

    RUST_LOG=multi_paxos=info cargo run --example start_learner -- <learner_uid> Config

See the file [`examples/start_learner.rs`](./examples/start_learner.rs) for more info.

### Examples

You can also run a simulation of a distributed system, where processes try to agree on a value proposed by a client, locally, by executing the following command:

    RUST_LOG=multi_paxos=info cargo run --example simulate

If you don't want all the logging messages, you can simply do

    cargo run --example simulate

Note: this example may not work in all systems, given that it uses some non-portable features, that are only available in certain operating systems. In a Mac OS X (and BSD-based OSes), it should work. See this [Stack Overflow post](https://stackoverflow.com/q/14388706/3924118).

## Bugs

- Not all tests are passing, IF the number of proposals for each client is greater, say, than 100-200.
    - I am not sure if this is a problem related to the implementation of the algorithm itself or related to the implementation of the struct which is responsible for sending and receiving messages using the UDP sockets (i.e., some kind of "buffering" problem?)
    - Anyway, I'm not sure if the implementation would always pass the tests if the number of client proposals is 100 or less.

## TODO

- Reimplement the tests in Rust (if possible)
- Implement leader election (to ensure progress)
- Remove dependency on the crate uuid (and its ability to generate UUIDs) to have correct executions
- Remove dependency on the input of the user regarding the unique ids of the nodes to have correct execution
- Allow types, like strings, to be agreed on (and not just numbers).
- Refactor the code to increase abstraction and flexibility
- Handle errors more appropriately
- Support command-line arguments in a more flexible way (maybe using a crate)
- Support IPv6

## Recommended Readings

- [Consensus](https://en.wikipedia.org/wiki/Consensus_(computer_science)), the Wikipedia article
- [Atomic Broadcast](https://en.wikipedia.org/wiki/Atomic_broadcast), the Wikipedia article
- [Paxos](https://en.wikipedia.org/wiki/Paxos_(computer_science)), the Wikipedia article
- [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf), a famous paper by [Leslie Lamport](https://en.wikipedia.org/wiki/Leslie_Lamport), the inventor of Paxos and winner of the 2013 [Turing Award](https://en.wikipedia.org/wiki/Turing_Award).
- [Understanding Paxos](https://understandingpaxos.wordpress.com/), an article by Tom Cocagne
- Section 1 (Introduction) and 2 (Implementation) of the paper [Multi-Paxos: An Implementation and Evaluation](https://pdfs.semanticscholar.org/b69c/9ea11b19ed17e253782e58b04ee2d6213579.pdf) by Hao Du and David J. St. Hilaire

## Other Readings

### Paxos

- http://www.fractalscape.org/files/paxos-family.pdf
- https://stackoverflow.com/q/5850487/3924118
- https://stackoverflow.com/q/10791825/3924118

### Multicast

- https://en.wikipedia.org/wiki/IP_multicast 
- https://stackoverflow.com/q/10692956/3924118
- https://www.tldp.org/HOWTO/Multicast-HOWTO-6.html

### Rust

- https://bluejekyll.github.io/blog/rust/2018/03/18/multicasting-in-rust.html
- https://github.com/rust-lang/rust-clippy/issues/1689 
- https://serde.rs/lifetimes.html

## Acknowledgements

A big thanks to all Rustaceans that helped me while developing the application. This was my first serious Rust program I developed, so I had to get used to the language first.