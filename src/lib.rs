use std::io;

use err_derive::Error;
use packet::{Packet, PacketType};
use tokio::io::{AsyncRead, AsyncWrite};

mod packet;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "{}", _0)]
    Io(#[error(source)] io::Error),

    #[error(display = "authentication failed")]
    Authentication,

    #[error(display = "command exceded maximum length")]
    CommandLength,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Connection<T> {
    io: T,
    default_packet_id: i32,
    current_packet_id: i32,
    max_payload_size: usize,
}

impl<T> Connection<T>
where
    T: Unpin + AsyncRead + AsyncWrite,
{
    pub fn builder() -> Builder {
        Builder {
            default_packet_id: 0,
            max_payload_size: 4096 - 10,
        }
    }

    pub async fn authenticate(&mut self, password: &str) -> Result<()> {
        self.send(PacketType::Auth, password.to_string()).await?;

        let packet = loop {
            let packet = self.receive_packet().await;

            if let Some(packet) = packet.ok() {
                if packet.packet_type == PacketType::AuthResponse {
                    break packet;
                }
            }
        };

        if packet.is_error() {
            Err(Error::Authentication)
        } else {
            Ok(())
        }
    }

    pub async fn execute_command(&mut self, command: &str) -> Result<Vec<String>> {
        if command.len() > self.max_payload_size {
            return Err(Error::CommandLength);
        }

        self.send(PacketType::ExecCommand, command.to_string())
            .await?;

        let response = self.recieve().await?;

        Ok(response)
    }

    async fn send(&mut self, packet_type: PacketType, payload: String) -> Result<()> {
        let packet = packet::Packet::new(self.new_packet_id(), packet_type, payload);
        self.send_packet(packet).await
    }

    async fn recieve(&mut self) -> Result<Vec<String>> {
        let mut responses = Vec::new();

        loop {
            let response = self.recieve_single_response().await?;
            responses.push(response);

            if let Some(last) = responses.last() {
                if last.is_empty() {
                    break;
                }
            }
        }

        Ok(responses)
    }

    async fn recieve_single_response(&mut self) -> Result<String> {
        let packet = self.receive_packet().await?;

        Ok(packet.payload.into())
    }

    async fn send_packet(&mut self, packet: Packet) -> Result<()> {
        match packet.write_to_io(&mut self.io).await {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::Io(err)),
        }
    }

    async fn receive_packet(&mut self) -> Result<Packet> {
        match Packet::read_from_io(&mut self.io).await {
            Ok(packet) => Ok(packet),
            Err(err) => Err(Error::Io(err)),
        }
    }

    fn new_packet_id(&mut self) -> i32 {
        let id = self.current_packet_id;

        self.current_packet_id = self
            .current_packet_id
            .checked_add(1)
            .unwrap_or(self.default_packet_id);

        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Builder {
    default_packet_id: i32,
    max_payload_size: usize,
}

impl Builder {
    pub fn default_packet_id(mut self, id: i32) -> Self {
        self.default_packet_id = id;
        self
    }

    pub fn max_payload_size(mut self, size: usize) -> Self {
        self.max_payload_size = size;
        self
    }

    pub fn build<T>(self, io: T) -> Connection<T>
    where
        T: Unpin + AsyncRead + AsyncWrite,
    {
        Connection {
            io,
            default_packet_id: self.default_packet_id,
            current_packet_id: self.default_packet_id,
            max_payload_size: self.max_payload_size,
        }
    }
}
