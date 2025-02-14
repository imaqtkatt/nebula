use std::{
  net::{Ipv4Addr, SocketAddr},
  sync::Arc,
};

use tokio::net::{TcpListener, UdpSocket};

mod device;
mod discovery;
mod packet;
mod serde;

const PORT: u16 = 35435;

const INTERFACE_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

const MULTI_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
const MULTICAST_ADDR: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(MULTI_ADDR), PORT);

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let socket = UdpSocket::bind((INTERFACE_ADDR, PORT)).await?;
  let socket = Arc::new(socket);
  socket.join_multicast_v4(MULTI_ADDR, INTERFACE_ADDR)?;
  socket.set_multicast_loop_v4(false)?;

  let _listener = TcpListener::bind((INTERFACE_ADDR, PORT)).await?;

  let node_id = uuid::Uuid::new_v4();

  let announcement = packet::Announcement {
    id: node_id,
    device: device::DEVICE,
  };

  let mut discovery = discovery::Discovery::new(socket);
  discovery.start(announcement).await?;

  if let Err(e) = discovery.block().await {
    eprintln!("Error: {e}");
  }

  Ok(())
}
