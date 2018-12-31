//! A module which contains the definition of the messages used in the Multi-Paxos algorithm.

// TODO: can the messages be structured in a cleaner (and still flexible) way?

use std::collections::HashMap;

use uuid::Uuid;

/// An enum which contains all types of messages which nodes, in the Paxos algorithm, can exchange.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message<T> {
    Phase0a(Request<T>),
    Phase0b(CatchUp),
    Phase0c(Report<T>),
    Phase1a(Preparation),
    Phase1b(Promise<T>),
    Phase1c(Nack),
    Phase2a(Proposal<T>),
    Phase2b(Acceptance<T>),
    Phase3(Learning<T>),
}

/// In phase 0, a client sends a proposal to a proposer, which needs to start the Paxos algorithm.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Request<T> {
    // The value which nodes need to agree on.
    pub value: T,

    // The unique identifier of the sender of this message (which is a client).
    pub sender_uuid: Uuid,
}

/// When a learner starts, it sends this message to the proposers to know about previously executed
/// Paxos instances.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct CatchUp {
    // The unique identifier of the Learner which sends this message.
    pub sender_uuid: Uuid,

    // 'l' for learner
    // 'p' for proposer
    pub sender_type: char,
}

/// The answer message to a CatchUp message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Report<T> {
    // The number of instances previously executed (according to the Proposer that sent this
    // message).
    pub num_of_instances: usize,

    // The learned values before the learner, with the unique identifier equal to the field
    // received_uid, was instantiated. It is actually a map between the Paxos instance numbers and
    // the associated learned values.
    pub learned_values: HashMap<usize, T>,

    // The unique identifier of the Proposer which sends this message.
    pub sender_uuid: Uuid,

    // The unique identifier of the Learner which receives this message.
    pub receiver_uuid: Uuid,
}

/// In phase 1a, c_rnd is sent from 1 proposer to ALL acceptors.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Preparation {
    // The highest-numbered round the proposer has started.
    pub c_rnd: usize,

    // The unique identifier of the sender of this message (which is a proposer).
    pub sender_uuid: Uuid,

    // The Paxos instance (or iteration) associated with this message.
    pub instance: usize,
}

/// In phase 1b, rnd, v_rnd and v_val is sent from 1 acceptor to 1 or more proposers.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Promise<T> {
    // The highest-numbered round the acceptor has PARTICIPATED in. It is initially 0. rnd is then
    // set to the c_rnd, sent in a Preparation message by some Proposer, such that c_rnd > rnd. So,
    // here, by "participate" we mean to send a Promise message to the proposals.
    pub rnd: usize,

    // The highest-numbered round the acceptor has CAST a vote. It is initially 0, but it eventually
    // corresponds to some c_rnd sent by a Proposer in a Proposal message, such that
    // c_rnd > self.rnd. In other words, v_rnd will be a number which is greater than any round the
    // acceptor has participated in. v_rnd is thus set only when the acceptor wants to send a Accept
    // message to the proposers, after having received enough Proposals. So, here, by casting a vote
    // we mean to send a Accept message to the proposers.
    pub v_rnd: usize,

    // The value voted by the acceptor in round v_rnd. It is initially None.
    pub v_val: Option<T>,

    // The unique identifier of the sender of this message (which is a acceptor).
    pub sender_uuid: Uuid,

    // The unique identifier of the interested receiver of the this message (which is a proposer).
    // It should match the field sender_uid of the Phase1a message.
    pub receiver_uuid: Uuid,

    pub instance: usize,
}

/// NACKs are optional in Paxos, but they can be used to inform other nodes of rejections.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Nack {
    // The v_rnd which caused the rejection of a c_rnd sent from a proposer to an acceptor in a
    // Preparation message.
    pub v_rnd: usize,

    // The unique identifier of the acceptor which rejects the c_rnd.
    pub sender_uuid: Uuid,

    // The unique identifier of the proposer to which this Nack message should be sent.
    pub receiver_uuid: Uuid,

    pub instance: usize,
}

/// In phase 2a, c_rnd and c_val is sent from 1 proposer to ALL acceptors.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Proposal<T> {
    pub c_rnd: usize,

    // The value that the proposer has picked for round c_rnd.
    pub c_val: Option<T>,

    pub sender_uuid: Uuid,

    pub instance: usize,
}

/// In phase 2b, v_rnd and v_val is sent from 1 acceptor to 1 or more proposers.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Acceptance<T> {
    pub v_rnd: usize,

    pub v_val: Option<T>,

    pub sender_uuid: Uuid,

    // It should match the field sender_uid of the Phase2a message.
    pub receiver_uuid: Uuid,

    pub instance: usize,
}

/// In phase 3, the proposers send the decided value to the learners.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Learning<T> {
    pub learned_value: T,

    pub sender_uuid: Uuid,

    pub instance: usize,
}