use crate::{
  device::Device,
  serde::{Deserialize, Serialize},
};

#[derive(Clone, Copy, Debug)]
pub struct Announcement {
  pub id: uuid::Uuid,
  pub device: Device,
}

const NEBULA: &[u8] = b"nebula";

impl Serialize for Announcement {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
    &self,
    wr: &mut W,
  ) -> std::io::Result<()> {
    wr.write_all(NEBULA).await?;
    self.id.serialize(wr).await?;
    self.device.serialize(wr).await
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
      Device::Macos => wr.write_u8(Device::TAG_MACOS).await,
      Device::Windows => wr.write_u8(Device::TAG_WINDOWS).await,
      Device::Linux => wr.write_u8(Device::TAG_LINUX).await,
      Device::Unknown => wr.write_u8(Device::TAG_UNKNOWN).await,
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

    Ok(Self { id, device })
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
    use std::io::{Error, ErrorKind};

    let tag = rd.read_u8().await?;

    let device = match tag {
      Device::TAG_MACOS => Device::Macos,
      Device::TAG_WINDOWS => Device::Windows,
      Device::TAG_LINUX => Device::Linux,
      Device::TAG_UNKNOWN => Device::Unknown,
      _ => Err(Error::new(ErrorKind::InvalidData, "invalid device tag"))?,
    };

    Ok(device)
  }
}
