use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{net::UdpSocket, sync::RwLock};

use crate::{
  device::Device,
  packet,
  serde::{Deserialize, Serialize},
  MULTICAST_ADDR,
};

pub type Peers = Arc<RwLock<HashMap<uuid::Uuid, Peer>>>;

const PEER_TIMEOUT_SECS: u64 = 40;

pub struct Discovery {
  socket: Arc<UdpSocket>,
  peers: Peers,
  announce: Option<tokio::task::JoinHandle<()>>,
  discover: Option<tokio::task::JoinHandle<()>>,
  remove: Option<tokio::task::JoinHandle<()>>,
}

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
  pub fn new(socket: Arc<UdpSocket>, peers: Peers) -> Self {
    Self {
      socket,
      peers,
      announce: None,
      discover: None,
      remove: None,
    }
  }
}

impl Discovery {
  pub async fn start(&mut self, announcement: packet::Announcement) -> std::io::Result<()> {
    self.announce = Some(self.announce(announcement).await?);
    self.discover = Some(self.discover());
    self.remove = Some(self.remove());

    Ok(())
  }

  async fn announce(
    &self,
    announcement: packet::Announcement,
  ) -> std::io::Result<tokio::task::JoinHandle<()>> {
    let mut buf = vec![];
    announcement.serialize(&mut buf).await?;

    let socket = Arc::clone(&self.socket);

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

  fn discover(&self) -> tokio::task::JoinHandle<()> {
    let socket = Arc::clone(&self.socket);
    let peers = Arc::clone(&self.peers);

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

        Self::upsert_peer(Arc::clone(&peers), announcement, addr).await;
      }
    })
  }

  async fn upsert_peer(
    peers: Peers,
    announcement: packet::Announcement,
    addr: std::net::SocketAddr,
  ) {
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

  fn remove(&self) -> tokio::task::JoinHandle<()> {
    let peers = Arc::clone(&self.peers);

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

  pub async fn block(self) -> Result<(), tokio::task::JoinError> {
    match (self.announce, self.discover, self.remove) {
      (Some(an), Some(di), Some(re)) => an.await.and(di.await).and(re.await),
      _ => panic!("invalid discovery state"),
    }
  }
}
