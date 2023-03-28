mod common;
mod location;
mod msg;

use std::str;

use super::Decoded;
use location::LocationMsg;

pub enum Msg {
    Location(Decoded<LocationMsg>),
}

/// Returns the string between the H02 message prefix "*HQ"
/// and suffix "#", fails if they are not found.
fn get_message_frame(h02_str: String) -> Result<String, String> {
    const MSG_PREFIX: &str = "*HQ";
    const MSG_SUFFIX: &str = "#";

    let s = h02_str
        .find(MSG_PREFIX)
        .ok_or("required *HQ message prefix not present")?
        + MSG_PREFIX.len();

    let e = h02_str
        .find(MSG_SUFFIX)
        .ok_or("required # message suffix not present")?;

    Ok(h02_str[s..e].to_string())
}

pub fn decode(packets: &[u8]) -> Result<Msg, String> {
    let packets = str::from_utf8(packets)
        .or(Err("failed to read packets as utf8"))?
        .to_string();

    let message_frame = get_message_frame(packets)?;

    let parts: Vec<&str> = message_frame.split(",").filter(|x| x.len() > 0).collect();

    if parts.len() < 2 {
        return Err("cannot get message type to decode as".to_string());
    }

    let message_type = *parts.get(1).ok_or("error getting message type")?;

    return match message_type {
        msg::LOCATION => Ok(Msg::Location(location::from_parts(parts)?)),
        _ => Err("unknown message type".to_string()),
    };
}
