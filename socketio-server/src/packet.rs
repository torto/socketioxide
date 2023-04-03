use itertools::Itertools;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::debug;

use crate::errors::Error;

/// The socket.io packet type.
/// Each packet has a type and a namespace
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Packet<T> {
    pub inner: PacketData<T>,
    pub ns: String,
}

impl Packet<ConnectPacket> {
    pub fn connect(ns: String, sid: i64) -> Self {
        Self {
            inner: PacketData::Connect(Some(ConnectPacket {
                sid: sid.to_string(),
            })),
            ns,
        }
    }
}

impl<T> Packet<T> {
    pub fn event(ns: String, e: String, data: T) -> Self {
        Self {
            inner: PacketData::Event(e, data),
            ns,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PacketData<T> {
    Connect(Option<T>),
    Disconnect,
    Event(String, T),
    Ack(i64),
    ConnectError(ConnectErrorPacket),
    BinaryEvent(String, T, Vec<Vec<u8>>),
    BinaryAck(T, Vec<Vec<u8>>),
}

impl<T> PacketData<T> {
    fn index(&self) -> u8 {
        match self {
            PacketData::Connect(_) => 0,
            PacketData::Disconnect => 1,
            PacketData::Event(_, _) => 2,
            PacketData::Ack(_) => 3,
            PacketData::ConnectError(_) => 4,
            PacketData::BinaryEvent(_, _, _) => 5,
            PacketData::BinaryAck(_, _) => 6,
        }
    }
}

impl<T> TryInto<String> for Packet<T>
where
    T: Serialize,
{
    type Error = Error;

    fn try_into(self) -> Result<String, Self::Error> {
        let mut res = self.inner.index().to_string();
        if !self.ns.is_empty() && self.ns != "/" {
            res.push_str(&format!("{},", self.ns));
        }

        match self.inner {
            PacketData::Connect(Some(data)) => res.push_str(&serde_json::to_string(&data)?),
            PacketData::ConnectError(data) => res.push_str(&serde_json::to_string(&data)?),
            PacketData::Event(event, data) => {
                res.push_str(&serde_json::to_string(&(event, &data))?)
            }
            PacketData::Ack(_) => todo!(),
            PacketData::BinaryEvent(_, _, _) => todo!(),
            PacketData::BinaryAck(_, _) => todo!(),
            _ => {}
        };
        Ok(res)
    }
}

fn get_packet<T>(data: &str) -> Result<Option<T>, Error>
where
    T: DeserializeOwned,
{
    debug!("Deserializing packet: {:?}", data);
    let packet = if data.is_empty() {
        None
    } else {
        Some(serde_json::from_str(data)?)
    };
    Ok(packet)
}

/// Deserialize a packet from a string
/// The string should be in the format of:
/// ```text
/// <packet type>[<# of binary attachments>-][<namespace>,][<acknowledgment id>][JSON-stringified payload without binary]
/// + binary attachments extracted
/// ```
impl<T> TryFrom<String> for Packet<T>
where
    T: DeserializeOwned,
{
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut chars = value.chars();
        let index = chars.next().ok_or(Error::InvalidPacketType)?;
        //TODO: attachments
        let _attachments: u32 = chars
            .take_while_ref(|c| *c != '-' && c.is_digit(10))
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        let mut ns: String = chars
            .take_while_ref(|c| *c != ',' && *c != '{' && !c.is_digit(10))
            .collect();

        // If the namespace is not empty it has a `,` separator after
        if !ns.is_empty() {
            chars.next();
        } 
        if !ns.starts_with("/") {
            ns.insert(0, '/');
        }
        //TODO: ack
        let _ack: Option<i64> = chars
            .take_while_ref(|c| c.is_digit(10))
            .collect::<String>()
            .parse()
            .ok();

        let data = chars.as_str();
        let inner = match index {
            '0' => PacketData::Connect(get_packet(&data)?),
            '1' => PacketData::ConnectError(get_packet(&data)?.ok_or(Error::InvalidPacketType)?),
            '2' => PacketData::Disconnect,
            '3' => {
                let (event, payload): (String, T) =
                    get_packet(&data)?.ok_or(Error::InvalidPacketType)?;
                PacketData::Event(event, payload)
            }
            '4' => todo!(),
            '5' => todo!(),
            '6' => todo!(),
            _ => return Err(Error::InvalidPacketType),
        };

        Ok(Self { inner, ns })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Placeholder {
    #[serde(rename = "_placeholder")]
    placeholder: bool,
    num: u32,
}

/// Connect packet sent by the client
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectPacket {
    sid: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectErrorPacket {
    message: String,
}