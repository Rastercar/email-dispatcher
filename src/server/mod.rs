use std::{future::Future, marker::Send};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::UnboundedSender,
    task::JoinHandle,
};

use crate::queue::RmqMessage;

pub mod h02;

/// The buffer size to be used when reading tracker connections.
///
/// This is more than enough to handle all packets from all trackers,
/// if a connection sends a packet through TCP/UDP with more bytes than
/// this then its very unlikely to be a tracking device
pub const BUFFER_SIZE: usize = 512;

/// The maximun amount of undecodeable packets a tracker can send
/// before its connection should be dropped
pub const INVALID_PACKET_LIMIT: usize = 10;

/// A TCP handle recieves the tcp stream to handle and a unbounded sender
/// to send the decoded tracker events sent over the TCP connection (such
/// as a new position or tracker command response)
type TcpHandler<R> = fn(TcpStream, UnboundedSender<RmqMessage>) -> R;

/// Start a new tokio task that binds a TcpListener to addr and pass all
/// incoming connections to the the handler on another task.
pub fn start_tcp_listener(
    addr: &str,
    sender: UnboundedSender<RmqMessage>,
    handler: TcpHandler<impl Future<Output = ()> + 'static + Send>,
) -> JoinHandle<()> {
    let addr = addr.to_string();

    tokio::spawn(async move {
        let listener = TcpListener::bind(addr)
            .await
            .expect("failed to start TCP listener");

        println!("[TCP] listener started");

        loop {
            for (stream, _) in listener.accept().await {
                let c = sender.clone();
                tokio::spawn(handler(stream, c));
            }
        }
    })
}
