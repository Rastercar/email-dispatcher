use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;

use super::{BUFFER_SIZE, INVALID_PACKET_LIMIT};
use crate::decoder::{h02, Decoded};
use crate::queue::RmqMessage;

use serde::Serialize;

// TODO: MOVE ME TO RMQ MODULE
fn send_tracker_event<T>(evt: &Decoded<T>, sender: &UnboundedSender<RmqMessage>)
where
    T: Serialize,
{
    let routing_key = [
        evt.protocol.to_string(),
        evt.event_type.to_string(),
        evt.imei.clone(),
    ]
    .join(".");

    match serde_json::to_string(&evt.data) {
        Ok(serialized) => {
            println!("---------------");
            println!("{}", serialized);
            println!("{}", routing_key);
            println!("---------------");

            sender
                .send(RmqMessage {
                    routing_key,
                    body: String::from("asd"),
                })
                .unwrap(); // TODO: remove unwrap

            return;
            // send here
        }
        // TODO: log error to jaeger here ?
        Err(_) => (),
    };
}

fn handle_decoded_message(
    msg: &h02::Msg,
    sender: &UnboundedSender<RmqMessage>,
) -> Option<Box<[u8]>> {
    match msg {
        h02::Msg::Location(decoded) => {
            send_tracker_event(&decoded, sender);
            return decoded.response.clone();
        }
    }
}

pub async fn stream_handler(stream: TcpStream, sender: UnboundedSender<RmqMessage>) {
    let mut buffer = vec![0; BUFFER_SIZE];

    let (mut reader, mut writer) = io::split(stream);

    let mut invalid_packets_cnt: usize = 0;

    while let Ok(n) = reader.read(&mut buffer).await {
        if n == 0 {
            // EOF
            break;
        }

        let packets = &buffer[..n];

        match h02::decode(packets) {
            Ok(msg) => match handle_decoded_message(&msg, &sender) {
                None => (),

                // We intentionally block on write here because because writes rarelly happen (so blocking shouldnt be much of a problem)
                // and because some tracker models should recieve the response to their commands in order, so if a tracker sends a command
                // A and B reponses A1 and B1 should be in that order.
                Some(response) => match writer.write_all(&*response).await {
                    Ok(_) => (),

                    // writes to the tracker happen when responding to commands and failures
                    // are a really bad state, so for now assume the connection is unrecoverable
                    // and end it.
                    Err(_) => break,
                },
            },
            Err(_) => {
                invalid_packets_cnt = invalid_packets_cnt + 1;

                if invalid_packets_cnt >= INVALID_PACKET_LIMIT {
                    break;
                }
            }
        }
    }
}
