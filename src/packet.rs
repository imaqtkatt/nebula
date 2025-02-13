use crate::{
  device::Device,
  serde::{Deserialize, Serialize},
};

#[derive(Clone, Copy, Debug)]
pub struct Announcement {
  pub id: uuid::Uuid,
  pub device: Device,
  pub port: u16,
}

const NEBULA: &[u8] = b"nebula";

impl Serialize for Announcement {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
    &self,
    wr: &mut W,
  ) -> std::io::Result<()> {
    wr.write_all(NEBULA).await?;
    self.id.serialize(wr).await?;
    self.device.serialize(wr).await?;
    wr.write_u16(self.port).await
  }
}

impl Serialize for uuid::Uuid {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
    &self,
    wr: &mut W,
  ) -> std::io::Result<()> {
    wr.write_all(self.as_bytes()).await
  }
}

impl Serialize for Device {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
    &self,
    wr: &mut W,
  ) -> std::io::Result<()> {
    match self {
      Device::Macos => wr.write_u8(1).await,
      Device::Windows => wr.write_u8(2).await,
      Device::Linux => wr.write_u8(3).await,
      Device::Unknown => wr.write_u8(4).await,
    }
  }
}

impl Deserialize for Announcement {
  async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
    use std::io::{Error, ErrorKind};

    let mut buf = [0u8; 6];
    rd.read_exact(&mut buf).await?;

    if buf != NEBULA {
      Err(Error::new(ErrorKind::InvalidData, "invalid nebula packet"))?;
    }

    let id = uuid::Uuid::deserialize(rd).await?;
    let device = Device::deserialize(rd).await?;
    let port = rd.read_u16().await?;

    Ok(Self { id, device, port })
  }
}

impl Deserialize for uuid::Uuid {
  async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
    let mut buf = [0u8; 16];
    rd.read_exact(&mut buf).await?;
    Ok(Self::from_bytes(buf))
  }
}

impl Deserialize for Device {
  async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
    let tag = rd.read_u8().await?;

    let device = match tag {
      1 => Device::Macos,
      2 => Device::Windows,
      3 => Device::Linux,
      _ => todo!(),
    };

    Ok(device)
  }
}
