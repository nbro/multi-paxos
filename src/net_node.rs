//! A module which contains the definition of a struct which can be used to send or receive messages
//! using a UDP socket.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use bincode::{deserialize, serialize};
use net2::UdpBuilder;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::message::Message;

/// A struct which can be used to send to or receive from a UDP socket.
pub struct NetNode<T> {
    udp_socket_sender: UdpSocket,

    udp_socket_receiver: UdpSocket,

    // Dummy data that is associated with the type of the value that a client initially proposes.
    value: PhantomData<T>,
}

impl<T> NetNode<T>
    where T: Serialize + DeserializeOwned + Clone + Debug,
{
    // TODO: verify that this can be deployed on several distributed machines.
    pub fn new(multicast_address_v4: &SocketAddrV4) -> Self {
        // Create the UdpSocket to send messages to other sockets. This socket does not have to bind
        // to a specific port, but just to one available, hence we use 0 as the port, which is used
        // to do that.
        let udp_socket_sender = UdpSocket::bind("0.0.0.0:0").expect("Could not bind to address");

        // TODO: do I need this?
        udp_socket_sender.set_multicast_loop_v4(true).expect("set_multicast_loop_v4 call failed");

        // Create a UdpSocket to receive messages from other sockets on the same address as the
        // multicast group one.
        let udp_socket_receiver = UdpBuilder::new_v4()
            .expect("Could not construct UdpBuilder")
            // Multiple sockets could bind to the same multicast group address, so we need this.
            .reuse_address(true)
            .expect("Could not reuse address")
            // Bind the receiver socket to the same host as the multicast group.
            .bind(multicast_address_v4)
            .expect("Could not bind to address");

        // Let the socket that wants to receive messages join its corresponding multicast group.
        udp_socket_receiver
            .join_multicast_v4(&multicast_address_v4.ip(), &Ipv4Addr::UNSPECIFIED)
            .expect("Could not join multicast group");

        NetNode { udp_socket_sender, udp_socket_receiver, value: PhantomData }
    }

    /// Sends the message m to the socket with address destination_address.
    pub fn send(&self, m: Message<T>, destination_address: &SocketAddrV4) {
        let encoded: Vec<u8> = serialize(&m).expect("Could not serialize the message m");

        self.udp_socket_sender
            .send_to(&encoded[..], destination_address)
            .expect("Could not send data");
    }

    /// Receives a message using the socket which listens on the address multicast_address_v4, given
    /// as parameter to the new function.
    pub fn receive(&self) -> Message<T> {
        // TODO: what's the required size of data_received?
        let mut data_received = vec![0; 16384];

        let (number_of_bytes, _src_addr) = self
            .udp_socket_receiver
            .recv_from(&mut data_received)
            .expect("Could not receive data");

        deserialize(&data_received[..number_of_bytes]).expect("Could not deserialize received data")
    }
}
