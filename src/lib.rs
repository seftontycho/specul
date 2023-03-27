//! A simple client for the [RCON](https://developer.valvesoftware.com/wiki/Source_RCON_Protocol) protocol.
//! # Example
//! ```no_run
//! use rcon::{Connection, ConnectionBuilder};
//! use tokio::net::TcpStream;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!    let mut connection = ConnectionBuilder::default()
//!       .password("password")
//!      .build(TcpStream::connect("0.0.0.0:27015").await?);
//!
//!     connection.authenticate().await?;
//!
//!     let response = connection.execute_command("status").await?;
//!
//!     println!("{:?}", response);
//!
//!   Ok(())
//! }

use std::io;

use derive_builder::Builder;
use err_derive::Error;
use packet::{Packet, PacketType};
use tokio::io::{AsyncRead, AsyncWrite};

mod packet;

/// An error that can occur when communicating with the server.
#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "{}", _0)]
    Io(#[error(source)] io::Error),

    #[error(display = "authentication failed")]
    Authentication,

    #[error(display = "payload size exceeded")]
    PayloadSize,
}

/// A specialized [`Result`](std::result::Result) type for RCON operations.
pub type Result<T> = std::result::Result<T, Error>;

/// A connection to a RCON server.
/// Can be constructed with any type that implements
/// [`AsyncRead`](tokio::io::AsyncRead) and [`AsyncWrite`](tokio::io::AsyncWrite).
#[derive(Debug, Builder)]
pub struct Connection<T> {
    io: T,
    default_packet_id: i32,
    #[builder(default = "0", setter(skip))]
    current_packet_id: i32,
    max_payload_size: usize,
    multiple_responses: bool,
}

impl<T> Connection<T>
where
    T: Unpin + AsyncRead + AsyncWrite,
{
    /// Authenticates with the server.
    pub async fn authenticate(&mut self, password: &str) -> Result<()> {
        self.send(PacketType::Authentication, password.to_string())
            .await?;

        let packet = loop {
            let packet = self.receive_packet().await;

            if let Some(packet) = packet.ok() {
                if packet.packet_type == PacketType::AuthenticationResponse {
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

    /// Executes a command on the server.
    pub async fn execute_command(&mut self, command: &str) -> Result<Vec<String>> {
        if command.len() > self.max_payload_size {
            return Err(Error::PayloadSize);
        }

        self.send(PacketType::Message, command.to_string()).await?;

        let response = self.recieve().await?;

        Ok(response)
    }

    /// Sends a payload to the server.
    pub async fn send(&mut self, packet_type: PacketType, payload: String) -> Result<()> {
        let packet = packet::Packet::new(self.new_packet_id(), packet_type, payload);
        self.send_packet(packet).await
    }

    /// Receives payload(s) from the server.
    pub async fn recieve(&mut self) -> Result<Vec<String>> {
        if self.multiple_responses {
            self.recieve_multi_response().await
        } else {
            let reponse = self.recieve_single_response().await?;
            Ok(vec![reponse])
        }
    }

    /// Receives multiple payloads from the server.
    pub async fn recieve_multi_response(&mut self) -> Result<Vec<String>> {
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

    /// Receives a single payload from the server.
    pub async fn recieve_single_response(&mut self) -> Result<String> {
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
