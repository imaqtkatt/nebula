use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{net::UdpSocket, sync::RwLock};

use crate::{
  device::Device,
  event::{self, NebulaEvent},
  packet,
  serde::{Deserialize, Serialize},
  MULTICAST_ADDR_V4, MULTICAST_ADDR_V6,
};

pub struct Discovery<const IPV6: bool> {
  announce: tokio::task::JoinHandle<()>,
  discover: tokio::task::JoinHandle<()>,
  remove: tokio::task::JoinHandle<()>,
}

pub type Peers = Arc<RwLock<HashMap<uuid::Uuid, Peer>>>;

const PEER_TIMEOUT_SECS: u64 = 20;

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

impl<const IPV6: bool> Discovery<IPV6> {
  pub async fn init(
    socket: Arc<UdpSocket>,
    announcement: packet::Announcement,
    event_sender: &event::EventSender,
    peers: Peers,
  ) -> std::io::Result<Self> {
    Ok(Self {
      announce: announce::<IPV6>(Arc::clone(&socket), announcement).await?,
      discover: discover(
        Arc::clone(&socket),
        event_sender.clone(),
        Arc::clone(&peers),
      ),
      remove: remove(Arc::clone(&peers), event_sender.clone()),
    })
  }
}

impl<const IPV6: bool> Discovery<IPV6> {
  pub async fn block(self) -> Result<(), tokio::task::JoinError> {
    self
      .announce
      .await
      .and(self.discover.await)
      .and(self.remove.await)
  }
}

async fn announce<const IPV6: bool>(
  socket: Arc<UdpSocket>,
  announcement: packet::Announcement,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
  let mut buf = vec![];
  announcement.serialize(&mut buf).await?;

  let multi_addr = if IPV6 {
    MULTICAST_ADDR_V6
  } else {
    MULTICAST_ADDR_V4
  };

  let handle = tokio::spawn(async move {
    loop {
      if let Err(e) = socket.send_to(&buf, multi_addr).await {
        eprintln!("Error while sending announcement: {e}");
      }

      tokio::time::sleep(Duration::from_secs(5)).await;
    }
  });

  Ok(handle)
}

fn discover(
  socket: Arc<UdpSocket>,
  event_sender: event::EventSender,
  peers: Peers,
) -> tokio::task::JoinHandle<()> {
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

      upsert_peer(Arc::clone(&peers), announcement, addr, event_sender.clone()).await;
    }
  })
}

async fn upsert_peer(
  peers: Peers,
  announcement: packet::Announcement,
  addr: std::net::SocketAddr,
  event_sender: event::EventSender,
) {
  let mut peers = peers.write().await;

  println!("upserting peer, announcement = {announcement:?}, addr = {addr}");

  let peer = match peers.entry(announcement.id) {
    std::collections::hash_map::Entry::Occupied(occupied_entry) => {
      let peer = occupied_entry.into_mut();
      peer.last_seen = tokio::time::Instant::now();
      peer
    }
    std::collections::hash_map::Entry::Vacant(vacant_entry) => {
      let peer = Peer::new(announcement.id, announcement.device, addr);
      vacant_entry.insert(peer)
    }
  };

  if let Err(e) = event_sender.send(NebulaEvent::PeerUpserted {
    peer: (*peer).into(),
  }) {
    eprintln!("Error while sending event: {e}");
  }
}

fn remove(peers: Peers, event_sender: event::EventSender) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_secs(10)).await;

      let mut peers_write = peers.write().await;

      let peers_to_remove: Vec<_> = peers_write
        .iter()
        .filter(|(_, peer)| peer.last_seen.elapsed() > Duration::from_secs(PEER_TIMEOUT_SECS))
        .map(|(id, _)| *id)
        .collect();

      for peer_id in peers_to_remove {
        if let Err(e) = event_sender.send(NebulaEvent::PeerDisconnected { id: peer_id }) {
          eprintln!("Error while sending event: {e}");
        }
        peers_write.remove(&peer_id);
      }
    }
  })
}
