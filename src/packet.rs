use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum PacketType {
    Authentication,
    AuthenticationResponse,
    Message,
    Response,
    Unknown(i32),
}

impl PacketType {
    fn parse(value: i32, response: bool) -> Self {
        match value {
            3 => PacketType::Authentication,
            2 if response => PacketType::AuthenticationResponse,
            2 => PacketType::Message,
            0 => PacketType::Response,
            _ => PacketType::Unknown(value),
        }
    }

    fn format(&self) -> i32 {
        match self {
            PacketType::Authentication => 3,
            PacketType::AuthenticationResponse => 2,
            PacketType::Message => 2,
            PacketType::Response => 0,
            PacketType::Unknown(value) => *value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Packet {
    pub id: i32,
    pub length: i32,
    pub packet_type: PacketType,
    pub payload: String,
}

impl Packet {
    pub fn new(id: i32, packet_type: PacketType, payload: String) -> Self {
        let length = 10 + payload.len() as i32;
        Packet {
            id,
            length,
            packet_type,
            payload,
        }
    }

    pub fn is_error(&self) -> bool {
        self.id < 0
    }

    pub(crate) async fn write_to_io<T: Unpin + AsyncWrite>(&self, io: &mut T) -> io::Result<()> {
        let mut writer = BufWriter::new(io);

        writer.write_i32_le(self.length).await?;
        writer.write_i32_le(self.id).await?;
        writer.write_i32_le(self.packet_type.format()).await?;
        writer.write_all(self.payload.as_bytes()).await?;

        // Ending empty strings
        writer.write_all(&[0x00, 0x00]).await?;

        writer.flush().await?;

        Ok(())
    }

    pub(crate) async fn read_from_io<T: Unpin + AsyncRead>(io: &mut T) -> io::Result<Self> {
        let mut reader = BufReader::new(io);

        let length = reader.read_i32_le().await?;
        let id = reader.read_i32_le().await?;
        let packet_type = PacketType::parse(reader.read_i32_le().await?, true);

        let mut buffer = vec![0; length as usize - 10];
        reader.read_exact(&mut buffer).await?;

        let payload = String::from_utf8(buffer);

        let payload = match payload {
            Ok(payload) => payload,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 payload",
            ))?,
        };

        // Skip ending empty strings
        reader.read_u16_le().await?;

        Ok(Packet {
            id,
            length,
            packet_type,
            payload,
        })
    }
}
