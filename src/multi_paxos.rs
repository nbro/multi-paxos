//! The module that contains the structs representing clients, proposers, acceptors and learners of
//! the Multi-Paxos algorithm. It also contains the main logic of the algorithm.
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddrV4;

use log::Level;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::message::{
    Acceptance, CatchUp, Learning, Message, Preparation, Promise, Proposal, Report, Request,
};
use crate::net_node::NetNode;

/// Implement this trait if you are a process which needs to run in a infinite loop, while receiving
/// and sending messages.
pub trait Runnable {
    fn run(&mut self);
}

/// The struct representing the client in the Paxos algorithm.
pub struct Client<T> {
    // Every process has an associated universal unique identifier number.
    // https://en.wikipedia.org/wiki/Universally_unique_identifier
    uuid: Uuid,

    id: usize,

    node: NetNode<T>,

    proposers_address: SocketAddrV4,
}

impl<T> Client<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    pub fn new(id: usize, clients_address: SocketAddrV4, proposers_address: SocketAddrV4) -> Self {
        Client {
            uuid: Uuid::new_v4(),
            id,
            node: NetNode::new(&clients_address),
            proposers_address,
        }
    }

    pub fn request(&self, value: T) {
        let m = Message::Phase0a::<T>(Request {
            value,
            sender_uuid: self.uuid,
        });

        self.node.send(m.clone(), &self.proposers_address);

        if log_enabled!(Level::Info) {
            info!(
                "[C={:?}] {:?} sent to {:?}.",
                self.id, m, self.proposers_address
            );
        }
    }
}

/// In the Multi-Paxos algorithm, a proposer can participate in several instances of the basic Paxos
/// algorithm (at the same time). Given that messages can be received out-of-order, we need to save
/// the state of all those instances, in order to decide what to do depending on the instance and
/// and its associated values. This struct contains the values, of a single proposer, which are
/// associated with 1 instance of the basic Paxos algorithm.
struct ProposerState<T> {
    // The value that this proposer initially wants to propose.
    value: Option<T>,

    // The highest-numbered round the proposer has started. This number is incremented in phase 1a.
    c_rnd: usize,

    // The value that the proposer has picked for round self.c_rnd. This value can be self.value or
    // a value which is sent from one acceptor, in a Promise message, to this Proposer.
    c_val: Option<T>,

    // In order to send a Proposal to the acceptors, the majority of the acceptors must have
    // responded, to the initial Preparation message, with a Promise message, which contains a rnd
    // field (which is the highest-numbered round the corresponding acceptor has PARTICIPATED in).
    // rnd_received is thus used to keep track of the rnd received from the acceptors. In order to
    // send a Proposal message to the acceptors, all rnd received must be equal to self.c_rnd.
    rnd_received: Vec<usize>,

    // A Proposer needs to propose the v_val with the associated highest v_rnd received. This field
    // is thus used to keep track of such v_rnd.
    highest_v_rnd_received: usize,

    // The v_val associated with self.highest_v_rnd_received. If self.highest_v_rnd_received == 0,
    // then this will be set to self.value, because, if self.highest_v_rnd_received == 0, it means
    // that acceptors are in the first round and have not yet received any proposal.
    associated_v_val_received: Option<T>,

    // In order to send a Learning message to the learners, the majority of the acceptors must have
    // responded, to the Proposal message, with an Acceptance message, which contains a v_rnd and
    // the corresponding v_val. More specifically, to send a Learning message to the learners, all
    // v_rnd in self.v_rnd_received must be equal to self.c_rnd.
    v_rnd_received: Vec<usize>,
}

// I had to implement Default manually. See https://github.com/rust-lang/rust/issues/45036.
impl<T> Default for ProposerState<T> {
    fn default() -> Self {
        ProposerState {
            value: None,
            c_rnd: 0,
            c_val: None,
            rnd_received: Vec::new(),
            highest_v_rnd_received: 0,
            associated_v_val_received: None,
            v_rnd_received: Vec::new(),
        }
    }
}

/// The struct representing the proposer in the Paxos algorithm.
pub struct Proposer<T> {
    uuid: Uuid,

    id: usize,

    // Each instance of the Paxos algorithm, in the Multi-Paxos algorithm, is associated with 1
    // ProposerState<T>. This is a map from each instance (of a basic Paxos algorithm), which is a
    // number, to the corresponding ProposerState<T> needed to complete that instance.
    proposer_states: HashMap<usize, ProposerState<T>>,

    majority_of_acceptors: usize,

    // The number of instances of the basic Paxos algorithm which are being keep track of.
    // Initially, this field is 0.
    num_of_instances: usize,

    // A map between basic Paxos instances and the associated learned values. Of course, when this
    // proposer starts, this map is empty.
    learned_values: HashMap<usize, T>,

    node: NetNode<T>,

    proposers_address: SocketAddrV4,

    acceptors_address: SocketAddrV4,

    learners_address: SocketAddrV4,
}

impl<T> Proposer<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    pub fn new(
        id: usize,
        proposers_address: SocketAddrV4,
        acceptors_address: SocketAddrV4,
        learners_address: SocketAddrV4,
        num_of_acceptors: usize,
    ) -> Self {
        Proposer {
            uuid: Uuid::new_v4(),
            id,
            proposer_states: HashMap::new(),
            majority_of_acceptors: num_of_acceptors / 2 + 1,
            num_of_instances: 0,
            learned_values: HashMap::new(),
            node: NetNode::new(&proposers_address),
            proposers_address,
            acceptors_address,
            learners_address,
        }
    }

    // Handlers

    /// Handles the Request message sent by a client to this proposer.
    fn handle_request(&mut self, request: Request<T>) {
        if log_enabled!(Level::Info) {
            info!("[P={:?}] I will handle {:?}.", self.id, request);
        }

        self.prepare(request.value);
    }

    /// Handles the CatchUp messages sent by the learners.
    fn handle_catch_up(&mut self, catch_up: CatchUp) {
        // If it was another proposer or a learner that sent the CatchUp message, then I will
        // report, otherwise, because the sender is self, nothing is done. So, this avoids
        // responding to a CatchUp message sent by itself: of course, this would be a useless
        // operation, and actually it would only mess up with the answers from the other proposers.
        if catch_up.sender_uuid != self.uuid {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will handle {:?}.", self.id, catch_up);
            }

            self.report(catch_up.sender_uuid, catch_up.sender_type);
        } else {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will NOT handle {:?}.", self.id, catch_up);
            }
        }
    }

    /// Handles the Report message sent by a proposer to this proposer.
    fn handle_report(&mut self, report: Report<T>) {
        // If the destination of the Report message, i.e. report.receiver_uid, is equal to self.uuid,
        // then it means that this Report message was sent to this proposer.
        if report.receiver_uuid == self.uuid {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will handle {:?}.", self.id, report);
            }

            self.num_of_instances = report.num_of_instances;
            self.learned_values = report.learned_values;
        } else {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will NOT handle {:?}.", self.id, report);
            }
        }
    }

    /// Handles the Promise message sent by an acceptor to this proposer.
    fn handle_promise(&mut self, promise: Promise<T>) {
        if promise.receiver_uuid == self.uuid {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will handle {:?}.", self.id, promise);
            }
            self.propose(promise.rnd, promise.v_rnd, promise.v_val, promise.instance);
        } else {
            if log_enabled!(Level::Info) {
                info!(
                    "[P={:?}] I will ignore {:?} for {:?}.",
                    self.id, promise, promise.receiver_uuid
                );
            }
        }
    }

    /// Handles the Acceptance message sent by an acceptor to this proposer.
    fn handle_acceptance(&mut self, acceptance: Acceptance<T>) {
        if log_enabled!(Level::Info) {
            info!("[P={:?}] I will handle {:?}.", self.id, acceptance);
        }

        match acceptance.v_val {
            Some(v) => self.decide(acceptance.v_rnd, v, acceptance.instance),
            _ => panic!("Logic error: contact the programmer."),
        }
    }

    // Senders

    /// A newly instantiated proposer can "catch up" the current state of the other proposers by
    /// sending to them a CatchUp message.
    fn catch_up(&self) {
        let m = Message::Phase0b(CatchUp {
            sender_uuid: self.uuid,
            sender_type: 'p',
        });

        if log_enabled!(Level::Info) {
            info!("[P={:?}] I will send {:?}.", self.id, m);
        }

        self.node.send(m, &self.proposers_address);
    }

    /// Sends a Report message to the learners which requested it using a CatchUp message.
    fn report(&self, sender_uid: Uuid, sender_type: char) {
        let m = Message::Phase0c::<T>(Report {
            num_of_instances: self.num_of_instances,
            learned_values: self.learned_values.clone(),
            sender_uuid: self.uuid,
            receiver_uuid: sender_uid,
        });

        if log_enabled!(Level::Info) {
            info!("[P={:?}] I will send {:?}.", self.id, m);
        }

        let destination_address = if sender_type == 'l' {
            self.learners_address
        } else {
            self.proposers_address
        };

        self.node.send(m, &destination_address);
    }

    /// Updates its internal, after having received a request by a client with a value, and sends a
    /// Preparation message to all acceptors.
    fn prepare(&mut self, value: T) {
        // Every time this function is called, a new instance of the basic Paxos algorithm is
        // (implicitly) started.
        self.num_of_instances += 1;

        // Get the ProposerState associated with the last or new instance of the basic Paxos
        // algorithm, which will be executed next.
        let state = self
            .proposer_states
            .entry(self.num_of_instances)
            .or_default();

        state.value = Some(value);

        // TODO: if self.id is not unique among all processes for an instance of Paxos, the
        // TODO: algorithm may not work properly. So, it should not rely on a unique
        // TODO: generation/increment of c_rnd based on self.id
        //
        // TODO: note that so far, prepare is called only once for each proposer for the same
        // TODO: instance. Therefore, (state.c_rnd + 1) * self.id should be unique, provided id is
        // TODO: also unique among the proposers (at least).
        state.c_rnd = (state.c_rnd + 1) * self.id;

        let m = Message::Phase1a::<T>(Preparation {
            c_rnd: state.c_rnd,
            sender_uuid: self.uuid,
            instance: self.num_of_instances,
        });

        if log_enabled!(Level::Info) {
            info!("[P={:?}] I will send {:?}.", self.id, m);
        }

        self.node.send(m, &self.acceptors_address);
    }

    /// Sends a Proposal message to the acceptors, if "enough" Promise messages have been received.
    fn propose(&mut self, rnd: usize, v_rnd: usize, v_val: Option<T>, instance: usize) {
        let state = self.proposer_states.entry(instance).or_default();

        state.rnd_received.push(rnd);

        // We keep track of the highest v_rnd (and the associated v_val) received from any of the
        // acceptors. See below the logic.
        if v_rnd > state.highest_v_rnd_received {
            state.highest_v_rnd_received = v_rnd;
            state.associated_v_val_received = v_val;
        }

        if state.rnd_received.len() < self.majority_of_acceptors {
            return;
        }

        if log_enabled!(Level::Info) {
            info!("[P={:?}] Majority of rnd received.", self.id);
        }

        // Furthermore, to proceed, the proposer must make sure that all rnd received are equal to
        // the c_rnd associated with the current instance of the basic Paxos algorithm.
        if state.rnd_received.iter().all(|&n| n == state.c_rnd) {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] All rnd received are equal to my c_rnd.", self.id);
            }

            // It means that no acceptor has previously participated in any round of the current
            // instance of the basic Paxos algorithm.
            if state.highest_v_rnd_received == 0 {
                // In that case, we use the value sent by the client in its request.
                state.c_val = state.value;
            } else {
                // Otherwise we use the value associated with the highest v_rnd received so far from
                // any of the acceptors.
                state.c_val = state.associated_v_val_received;
            }

            let m = Message::Phase2a::<T>(Proposal {
                c_rnd: state.c_rnd,
                c_val: state.c_val,
                sender_uuid: self.uuid,
                instance,
            });

            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will send {:?}.", self.id, m);
            }

            self.node.send(m, &self.acceptors_address);
        }

        // TODO: verify that the following program logic is correct.
        //
        // If the execution arrives here, it means that we have received rnd values from the
        // majority of the acceptors. These rnd values received are NOT necessarily ALL equal to
        // c_rnd.
        //
        // We should not clear this buffer at the end of the previous if block, because, suppose
        // that, at some point, rnd_received contains rnd values which are NOT ALL equal to c_rnd,
        // and we have NOT yet received rnd from the majority of the acceptors. In that case, none
        // of the if blocks above will be executed. Suppose that this function is called again and,
        // at that point, we have received rnd values from the majority of the acceptors. Even
        // though this may be the case, the previous if block will never be executed, because, even
        // though, at that point, we will have received rnd values from the majority of the
        // acceptors, they will NOT ALL be equal to c_rnd. After that point, we will possibly keep
        // receiving more rnd values from acceptors, but that will not change anything, because, if
        // we had "state.rnd_received.clear()" at the end of that if block, we would never clear
        // rnd_received, and thus rnd_received would never contain all rnd values equal c_rnd, and
        // so this proposer would never send back an answer to the acceptors (if this proposer is
        // called to handle a "promise" message). Note that rnd_received is only modified in this
        // function so far.
        //
        // If we clear this buffer here, we know that we have received rnd values from the majority
        // of the acceptors, but EITHER they were all equal to c_rnd or (exclusive or) NOT. If they
        // are all equal to c_rnd, then we have sent back an answer to the acceptors, otherwise we
        // have not. By clearing the buffer here, we can process other "promise" messages from the
        // acceptors. But, unless we need to send a new Preparation message to the acceptors, this
        // is not necessary. Right now, this implementation still doesn't support the re-sending of
        // Preparation messages in case a Nack is received.
        // state.rnd_received.clear();
    }

    /// Sends a Learning message to the learners, if "enough" Acceptance messages have been received
    /// from the acceptors.
    fn decide(&mut self, v_rnd: usize, v_val: T, instance: usize) {
        let state = self.proposer_states.entry(instance).or_default();

        state.v_rnd_received.push(v_rnd);

        if state.v_rnd_received.len() < self.majority_of_acceptors {
            return;
        }

        if log_enabled!(Level::Info) {
            info!("[P={:?}] Majority of messages received.", self.id);
        }

        // We keep track of the learned values so as to be able to answer to the CatchUp
        // messages sent by the learners. We need to store v_val here and not inside the next if
        // statement, because the next if statement may not be executed. Anyway, at this point,
        // v_val needs to be a value which learners need to know: it can or not be equal to
        // state.c_rnd.
        if let Some(v) = self.learned_values.insert(instance, v_val) {
            assert_eq!(
                v, v_val,
                "Bug: previously known v_val is not equal to current one for the same instance"
            );
        }

        if state.v_rnd_received.iter().all(|&n| n == state.c_rnd) {
            if log_enabled!(Level::Info) {
                info!(
                    "[P={:?}] All v_rnd received are equal to my c_rnd.",
                    self.id
                );
            }

            assert_eq!(
                v_val,
                state.c_val.unwrap(),
                "Bug: v_val should be equal to c_val to decide"
            );

            let m = Message::Phase3::<T>(Learning {
                learned_value: v_val,
                sender_uuid: self.uuid,
                instance,
            });

            if log_enabled!(Level::Info) {
                info!("[P={:?}] I will send {:?}.", self.id, m);
            }

            // We can send the message to the learners multiple times, because, once we have
            // received the majority of the messages containing v_rnd (and all v_rnd == c_rnd), then
            // all subsequent calls to this self.decide function will trigger this call too. Anyway,
            // we just need the majority and thus to send this message once.
            self.node.send(m, &self.learners_address);
        }

        // TODO: verify that this statement should be here.
        // state.v_rnd_received.clear();
    }
}

impl<T> Runnable for Proposer<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    fn run(&mut self) {
        self.catch_up();

        loop {
            if log_enabled!(Level::Info) {
                info!("[P={:?}] Proposer waiting...", self.id);
            }

            let m = self.node.receive();

            match m {
                Message::Phase0a::<T>(request) => self.handle_request(request),
                Message::Phase0b(catch_up) => self.handle_catch_up(catch_up),
                Message::Phase0c::<T>(report) => self.handle_report(report),
                Message::Phase1b::<T>(promise) => self.handle_promise(promise),
                Message::Phase2b::<T>(acceptance) => self.handle_acceptance(acceptance),
                _ => info!(
                    "[P={:?}] Unexpected message received. I'll ignore it.",
                    self.id
                ),
            }
        }
    }
}

/// In the Multi-Paxos algorithm, an acceptor can participate in several instances of the basic
/// Paxos algorithm (at the same time). Given that messages can be received out-of-order, we need to
/// save the state of all those instances, in order to decide what to do depending on the instance
/// and its associated values. This struct contains the values, of a single acceptor, which are
/// associated with 1 instance of the basic Paxos algorithm.
struct AcceptorState<T> {
    // The highest-numbered round the acceptor has PARTICIPATED in. It is initially 0. rnd is then
    // set to the c_rnd, sent in a Preparation message by some Proposer, such that c_rnd > rnd. So,
    // here, by "participate" we mean to send a Promise message to the proposals.
    rnd: usize,

    // The highest-numbered round the acceptor has CAST a vote. It is initially 0, but it eventually
    // corresponds to some c_rnd sent by a Proposer in a Proposal message, such that
    // c_rnd > self.rnd. In other words, v_rnd will be a number which is greater than any round the
    // acceptor has participated in. v_rnd is thus set only when the acceptor wants to send a
    // Acceptance message to the proposers, after having received enough Proposals. So, here, by
    // casting a vote we mean to send a Acceptance message to the proposers.
    v_rnd: usize,

    // The value voted by the acceptor in round v_rnd. It is initially None.
    v_val: Option<T>,
}

// I had to implement Default manually. See https://github.com/rust-lang/rust/issues/45036.
impl<T> Default for AcceptorState<T> {
    fn default() -> Self {
        AcceptorState {
            rnd: 0,
            v_rnd: 0,
            v_val: None,
        }
    }
}

/// The struct representing the acceptor in the Paxos algorithm.
pub struct Acceptor<T> {
    uuid: Uuid,

    id: usize,

    // Each instance of the Paxos algorithm, in the Multi-Paxos algorithm, is associated with 1
    // AcceptorState<T>. This is a map from each instance (of a basic Paxos algorithm), which is a
    // number, to the corresponding AcceptorState<T> needed to complete that instance.
    acceptor_states: HashMap<usize, AcceptorState<T>>,

    node: NetNode<T>,

    proposers_address: SocketAddrV4,
}

impl<T> Acceptor<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    pub fn new(
        id: usize,
        acceptors_address: SocketAddrV4,
        proposers_address: SocketAddrV4,
    ) -> Self {
        Acceptor {
            uuid: Uuid::new_v4(),
            id,
            acceptor_states: HashMap::new(),
            node: NetNode::new(&acceptors_address),
            proposers_address,
        }
    }

    // Handlers

    /// Handles the Preparation message sent by a proposer to this acceptor.
    fn handle_preparation(&mut self, preparation: Preparation) {
        if log_enabled!(Level::Info) {
            info!("[A={:?}] I will handle {:?}.", self.id, preparation);
        }

        self.promise(
            preparation.c_rnd,
            preparation.sender_uuid,
            preparation.instance,
        );
    }

    /// Handles the Proposal message sent by a proposer to this acceptor.
    fn handle_proposal(&mut self, proposal: Proposal<T>) {
        if log_enabled!(Level::Info) {
            info!("[A={:?}] I will handle {:?}.", self.id, proposal);
        }

        match proposal.c_val {
            Some(c_val) => self.accept(
                proposal.c_rnd,
                c_val,
                proposal.sender_uuid,
                proposal.instance,
            ),
            _ => panic!("Logic error: contact the programmer."),
        }
    }

    // Senders

    /// Sends a Promise message to one or more proposers, if c_rnd > rnd.
    fn promise(&mut self, c_rnd: usize, sender_uid: Uuid, instance: usize) {
        let state = self.acceptor_states.entry(instance).or_default();

        if c_rnd > state.rnd {
            // The promise.
            state.rnd = c_rnd;

            let m = Message::Phase1b::<T>(Promise {
                rnd: state.rnd,
                v_rnd: state.v_rnd,
                v_val: state.v_val, // The value it last accepted. It can be None.
                sender_uuid: self.uuid,
                receiver_uuid: sender_uid,
                instance,
            });

            if log_enabled!(Level::Info) {
                info!("[A={:?}] I will send {:?}.", self.id, m);
            }

            self.node.send(m, &self.proposers_address);
        } else {
            // TODO: send a NACK. Note that, to send a nack and handle nacks, we may need to change
            // TODO: the logic in several places. For example, we may need to clear buffers, once
            // TODO: a new preparation message is sent from the proposers to the acceptors.
            // TODO: note: sending and handling nacks should not be necessary for Paxos to work.
        }
    }

    /// Sends an Acceptance message to one or more proposers, if c_rnd >= rnd.
    fn accept(&mut self, c_rnd: usize, c_val: T, sender_uid: Uuid, instance: usize) {
        let state = self.acceptor_states.entry(instance).or_default();

        if c_rnd >= state.rnd {
            state.v_rnd = c_rnd;
            state.v_val = Some(c_val);

            let m = Message::Phase2b::<T>(Acceptance {
                v_rnd: state.v_rnd,
                v_val: state.v_val,
                sender_uuid: self.uuid,
                receiver_uuid: sender_uid,
                instance,
            });

            if log_enabled!(Level::Info) {
                info!("[A={:?}] I will send {:?}.", self.id, m);
            }

            self.node.send(m, &self.proposers_address);
        } else {
            // TODO: send a NACK. Note that, to send a nack and handle nacks, we may need to change
            // TODO: the logic in several places. For example, we may need to clear buffers, once
            // TODO: a new preparation message is sent from the proposers to the acceptors.
            // TODO: note: sending and handling nacks should not be necessary for Paxos to work.
        }
    }
}

impl<T> Runnable for Acceptor<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    fn run(&mut self) {
        loop {
            if log_enabled!(Level::Info) {
                info!("[A={:?}] Acceptor waiting...", self.id);
            }

            let m = self.node.receive();

            match m {
                Message::Phase1a::<T>(preparation) => self.handle_preparation(preparation),
                Message::Phase2a::<T>(proposal) => self.handle_proposal(proposal),
                _ => info!(
                    "[A={:?}] Unexpected message received. I'll ignore it.",
                    self.id
                ),
            }
        }
    }
}

/// The struct representing the learner in the Paxos algorithm.
pub struct Learner<T> {
    uuid: Uuid,

    id: usize,

    // A map between instance numbers (or ids) and the learned value during that instance.
    learned_values: HashMap<usize, T>,

    // The number of learned values printed to the standard output so far. This is used to print
    // the learned values in total order, that is, according to the increasing number of the
    // corresponding Paxos instance.
    num_of_instances: usize,

    node: NetNode<T>,

    // A learner needs to contact the proposers to ask them about previously executed basic Paxos
    // instances, in order to deliver the related learned values, before the future Paxos
    // instances that are eventually executed.
    proposers_address: SocketAddrV4,
}

impl<T> Learner<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    pub fn new(id: usize, learners_address: SocketAddrV4, proposers_address: SocketAddrV4) -> Self {
        Learner {
            uuid: Uuid::new_v4(),
            id,
            learned_values: HashMap::new(),
            num_of_instances: 1,
            node: NetNode::new(&learners_address),
            proposers_address,
        }
    }

    /// Tries to print the learned values that can be already printed, that is, the ones received in
    /// total order.
    fn print_learned_values(&mut self) {
        while self.learned_values.contains_key(&self.num_of_instances) {
            println!(
                "{:?}",
                self.learned_values.get(&self.num_of_instances).unwrap()
            );
            self.num_of_instances += 1;
        }
    }

    // Handlers

    /// Handles the Report message sent by a proposer to this learner.
    fn handle_report(&mut self, report: Report<T>) {
        if report.receiver_uuid == self.uuid {
            if log_enabled!(Level::Info) {
                info!("[L={:?}] Received {:?}.", self.id, report);
            }

            for (instance, learned_value) in report.learned_values {
                // It is possible that we receive the learned value associated with an instance from
                // more than one proposer.
                self.learned_values.insert(instance, learned_value);
            }

            self.print_learned_values();
        }
    }

    /// Handles the receipt of a Learning message sent by a proposer.
    fn handle_learning(&mut self, learning: Learning<T>) {
        if log_enabled!(Level::Info) {
            info!("[L={:?}] Received {:?}.", self.id, learning);
        }

        if let Some(v) = self
            .learned_values
            .insert(learning.instance, learning.learned_value)
        {
            // All proposers must learn the same value and send the same value to the learners.
            assert_eq!(
                v, learning.learned_value,
                "Bug: previously learned value is not equal to just learned one."
            );
        }

        self.print_learned_values();
    }

    // Senders

    /// Asks the proposers about previously executed basic Paxos instances and thus learned values.
    /// A learner, which is instantiated after some basic Paxos instances have been executed, must
    /// first know the learned values associated with these previously executed Paxos instances, so
    /// as to "deliver" the associated values before the values associated with the future Paxos
    /// instances that can eventually be executed.
    fn catch_up(&self) {
        let m = Message::Phase0b(CatchUp {
            sender_uuid: self.uuid,
            sender_type: 'l',
        });

        if log_enabled!(Level::Info) {
            info!("[L={:?}] I will send {:?}.", self.id, m);
        }

        self.node.send(m, &self.proposers_address);
    }
}

impl<T> Runnable for Learner<T>
where
    T: Serialize + DeserializeOwned + Copy + Clone + Debug + PartialEq,
{
    fn run(&mut self) {
        self.catch_up();

        loop {
            if log_enabled!(Level::Info) {
                info!("[L={:?}] Learner waiting...", self.id);
            }

            let m = self.node.receive();

            match m {
                Message::Phase0c::<T>(report) => self.handle_report(report),
                Message::Phase3::<T>(learning) => self.handle_learning(learning),
                _ => info!(
                    "[L={:?}] Unexpected message received. I'll ignore it.",
                    self.id
                ),
            }
        }
    }
}
