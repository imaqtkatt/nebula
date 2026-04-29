pub mod serialization;

use serialization::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)]
pub enum Device {
    Macos = 0x01,
    Windows,
    Linux,
    #[default]
    Unknown,
}

pub const CURRENT_DEVICE: Device = if cfg!(target_os = "macos") {
    Device::Macos
} else if cfg!(target_os = "windows") {
    Device::Windows
} else if cfg!(target_os = "linux") {
    Device::Linux
} else {
    Device::Unknown
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct PeerId(uuid::Uuid);

impl PeerId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Peer {
    pub id: PeerId,
    pub device: Device,
    pub addr: std::net::SocketAddr,
    pub last_seen: tokio::time::Instant,
}

pub type PeerMap = Arc<RwLock<HashMap<PeerId, Peer>>>;

impl Peer {
    pub fn new(id: PeerId, device: Device, addr: std::net::SocketAddr) -> Self {
        Self {
            id,
            device,
            addr,
            last_seen: tokio::time::Instant::now(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct AnnouncePacket {
    pub id: PeerId,
    pub device: Device,
}

impl Serialize for Device {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        wr.write_u8(*self as u8).await
    }
}

impl Deserialize for Device {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let device_tag = rd.read_u8().await?;
        let device = match device_tag {
            0x1 => Device::Macos,
            0x2 => Device::Windows,
            0x3 => Device::Linux,
            _ => Device::Unknown,
        };
        Ok(device)
    }
}

impl Serialize for PeerId {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.as_bytes();
        wr.write_all(bytes).await
    }
}

impl Deserialize for PeerId {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let mut buf = [0u8; 16];
        rd.read_exact(&mut buf).await?;

        let internal = uuid::Uuid::from_bytes(buf);
        Ok(PeerId(internal))
    }
}

const NEBU: &[u8; 4] = b"nebu";

#[allow(unused)]
const ANNOUNCE_PACKET_SIZE_BYTES: u8 = 4 + 16 + 1;

impl Serialize for AnnouncePacket {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        wr.write_all(NEBU).await?;
        self.id.serialize(wr).await?;
        self.device.serialize(wr).await?;
        Ok(())
    }
}

impl Deserialize for AnnouncePacket {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let mut nebu_buf = [0u8; 4];
        rd.read_exact(&mut nebu_buf).await?;
        if nebu_buf != *NEBU {
            Err(std::io::Error::other("is not a nebula packet"))?
        }

        let id = PeerId::deserialize(rd).await?;
        let device = Device::deserialize(rd).await?;

        Ok(Self { id, device })
    }
}

impl Serialize for Peer {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        self.id.serialize(wr).await?;
        self.device.serialize(wr).await?;

        SerializableSocketAddr::new(self.addr).serialize(wr).await?;

        let last_seen = self.last_seen.elapsed().as_secs().to_be_bytes();
        wr.write_all(&last_seen).await?;

        Ok(())
    }
}

pub struct SerializableSocketAddr(std::net::SocketAddr);

impl SerializableSocketAddr {
    pub fn new(addr: std::net::SocketAddr) -> Self {
        Self(addr)
    }

    pub fn get(&self) -> std::net::SocketAddr {
        self.0
    }
}

impl Serialize for SerializableSocketAddr {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        match self.0.ip() {
            std::net::IpAddr::V4(ipv4_addr) => {
                wr.write_all(b"V4").await?;
                let bits = ipv4_addr.to_bits();
                wr.write_u32(bits).await?;
            }
            std::net::IpAddr::V6(ipv6_addr) => {
                wr.write_all(b"V6").await?;
                let bits = ipv6_addr.to_bits();
                wr.write_u128(bits).await?;
            }
        }
        let port = self.0.port();
        wr.write_u16(port).await
    }
}

impl Deserialize for SerializableSocketAddr {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let mut tag_buf = [0u8; 2];
        rd.read_exact(&mut tag_buf).await?;

        let addr = match &tag_buf {
            b"V4" => {
                let bits = rd.read_u32().await?;
                let port = rd.read_u16().await?;

                std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::from_bits(bits),
                    port,
                ))
            }
            b"V6" => {
                let bits = rd.read_u128().await?;
                let port = rd.read_u16().await?;

                std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                    std::net::Ipv6Addr::from_bits(bits),
                    port,
                    0,
                    0,
                ))
            }
            _ => todo!(),
        };

        Ok(SerializableSocketAddr(addr))
    }
}
