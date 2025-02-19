use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{net::UdpSocket, sync::RwLock};

use crate::{
  device::Device,
  packet,
  serde::{Deserialize, Serialize},
  MULTICAST_ADDR,
};

pub struct Discovery {
  announce: tokio::task::JoinHandle<()>,
  discover: tokio::task::JoinHandle<()>,
  remove: tokio::task::JoinHandle<()>,
}

pub type Peers = Arc<RwLock<HashMap<uuid::Uuid, Peer>>>;

const PEER_TIMEOUT_SECS: u64 = 40;

#[derive(Clone, Copy, Debug)]
pub struct Peer {
  pub id: uuid::Uuid,
  pub device: Device,
  pub addr: std::net::SocketAddr,
  pub last_seen: tokio::time::Instant,
}

impl Peer {
  pub fn new(id: uuid::Uuid, device: Device, addr: std::net::SocketAddr) -> Self {
    Self {
      id,
      device,
      addr,
      last_seen: tokio::time::Instant::now(),
    }
  }
}

impl Discovery {
  pub async fn init(
    socket: Arc<UdpSocket>,
    announcement: packet::Announcement,
    peers: Peers,
  ) -> std::io::Result<Self> {
    Ok(Self {
      announce: announce(Arc::clone(&socket), announcement).await?,
      discover: discover(Arc::clone(&socket), Arc::clone(&peers)),
      remove: remove(Arc::clone(&peers)),
    })
  }
}

impl Discovery {
  pub async fn block(self) -> Result<(), tokio::task::JoinError> {
    self
      .announce
      .await
      .and(self.discover.await)
      .and(self.remove.await)
  }
}

async fn announce(
  socket: Arc<UdpSocket>,
  announcement: packet::Announcement,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
  let mut buf = vec![];
  announcement.serialize(&mut buf).await?;

  let handle = tokio::spawn(async move {
    loop {
      if let Err(e) = socket.send_to(&buf, MULTICAST_ADDR).await {
        eprintln!("Error while sending announcement: {e}");
      }

      tokio::time::sleep(Duration::from_secs(5)).await;
    }
  });

  Ok(handle)
}

fn discover(socket: Arc<UdpSocket>, peers: Peers) -> tokio::task::JoinHandle<()> {
  let mut buf = [0u8; 32];

  tokio::spawn(async move {
    loop {
      let (n, addr) = match socket.recv_from(&mut buf).await {
        Ok(value) => value,
        Err(e) => {
          eprintln!("Error while recv: {e}");
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

      upsert_peer(Arc::clone(&peers), announcement, addr).await;
    }
  })
}

async fn upsert_peer(peers: Peers, announcement: packet::Announcement, addr: std::net::SocketAddr) {
  let mut peers = peers.write().await;

  println!("upserting peer, announcement = {announcement:?}, addr = {addr}");

  // TODO: emit event?
  match peers.entry(announcement.id) {
    std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
      let peer = occupied_entry.get_mut();
      peer.last_seen = tokio::time::Instant::now();
    }
    std::collections::hash_map::Entry::Vacant(vacant_entry) => {
      vacant_entry.insert(Peer::new(announcement.id, announcement.device, addr));
    }
  }
}

fn remove(peers: Peers) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_secs(25)).await;

      let mut peers_write = peers.write().await;
      peers_write.retain(|_, peer| {
        if peer.last_seen.elapsed() > Duration::from_secs(PEER_TIMEOUT_SECS) {
          // TODO: emit event?
          false
        } else {
          true
        }
      });
    }
  })
}
