use core::fmt;

use serde::Serialize;

pub mod h02;

pub enum Protocol {
    H02,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Protocol::H02 => write!(f, "h02"),
        }
    }
}

pub enum TrackerEvent {
    Location,
}

impl fmt::Display for TrackerEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrackerEvent::Location => write!(f, "location"),
        }
    }
}

pub struct Decoded<T>
where
    T: Serialize,
{
    pub event_type: TrackerEvent,

    // imei of the tracker who sent the packet
    pub imei: String,

    // the decoded content
    pub data: T,

    // bytes to send in response to the tracker
    pub response: Option<Box<[u8]>>,

    // protocol used to decode the packet
    pub protocol: Protocol,
}
