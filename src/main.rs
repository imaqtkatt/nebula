use std::{
  net::{Ipv4Addr, SocketAddr},
  sync::Arc,
  time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, UdpSocket};

mod device;
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

  let listener = TcpListener::bind((INTERFACE_ADDR, 0)).await?;
  let port = listener.local_addr()?.port();

  let node_id = uuid::Uuid::new_v4();

  let announcement = packet::Announcement {
    id: node_id,
    port,
    device: device::DEVICE,
  };

  announce(Arc::clone(&socket), announcement).await?;
  let handle = discover(socket).await?;

  if let Err(e) = handle.await {
    eprintln!("Error: {e}");
  }

  Ok(())
}

async fn announce(
  socket: Arc<UdpSocket>,
  announcement: packet::Announcement,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
  socket.set_broadcast(true)?;

  let mut buf = vec![];
  announcement.serialize(&mut buf).await?;

  let handle = tokio::spawn(async move {
    loop {
      if let Err(e) = socket.send_to(&buf, MULTICAST_ADDR).await {
        eprintln!("Error while sending announcement: {e}")
      }

      tokio::time::sleep(Duration::from_secs(5)).await;
    }
  });

  Ok(handle)
}

async fn discover(socket: Arc<UdpSocket>) -> std::io::Result<tokio::task::JoinHandle<()>> {
  let mut buf = [0u8; 32];

  let handle = tokio::spawn(async move {
    loop {
      let (n, _addr) = match socket.recv_from(&mut buf).await {
        Ok(value) => value,
        Err(e) => {
          eprintln!("Error while receiving packet: {e}");
          continue;
        }
      };

      let mut buf = tokio::io::BufReader::new(&buf[..n]);
      let announcement = match packet::Announcement::deserialize(&mut buf).await {
        Ok(value) => value,
        Err(e) => {
          eprintln!("Error while deserializing packet: {e}");
          continue;
        }
      };

      // if announcement.id == id {
      //   continue;
      // }

      println!("Packet = {announcement:?}");
    }
  });

  Ok(handle)
}
