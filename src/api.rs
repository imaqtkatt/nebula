use std::time::Duration;

use base64::Engine;
use tokio::sync::Mutex;

use crate::{
  discovery::Peers,
  event,
  file_transfer::{self, FileTransferWsEvent},
};

#[derive(Clone)]
pub struct AppState {
  peers: Peers,
  pub file_transfer_event_emitter:
    ArcMutex<tokio::sync::mpsc::UnboundedSender<file_transfer::TransferData>>,
  pub event_receiver: ArcMutex<event::EventReceiver>,
}

type ArcMutex<T> = std::sync::Arc<Mutex<T>>;

impl AppState {
  pub fn new(
    peers: &Peers,
    file_transfer_event_emitter: tokio::sync::mpsc::UnboundedSender<file_transfer::TransferData>,
    event_receiver: event::EventReceiver,
  ) -> Self {
    Self {
      peers: std::sync::Arc::clone(peers),
      file_transfer_event_emitter: ArcMutex::new(Mutex::new(file_transfer_event_emitter)),
      event_receiver: std::sync::Arc::new(Mutex::new(event_receiver)),
    }
  }

  pub async fn get_peer(&self, peer_id: uuid::Uuid) -> Option<crate::discovery::Peer> {
    let peers_read = self.peers.read().await;
    peers_read.get(&peer_id).cloned()
  }

  pub async fn emit_file_transfer_event(
    &self,
    fte: FileTransferWsEvent,
  ) -> Result<(), tokio::sync::mpsc::error::SendError<file_transfer::TransferData>> {
    let Some(peer) = self.get_peer(fte.id).await else {
      unreachable!()
    };

    let addr = peer.addr;
    let data = base64::engine::general_purpose::STANDARD
      .decode(fte.data)
      .expect("decode base64");

    let guard = self.file_transfer_event_emitter.lock().await;
    guard.send(file_transfer::TransferData { addr, data })
  }
}

#[derive(Debug, serde::Serialize)]
pub struct JsonPeer {
  id: uuid::Uuid,
  device: crate::device::Device,
  last_seen: Duration,
}

impl From<crate::discovery::Peer> for JsonPeer {
  fn from(peer: crate::discovery::Peer) -> Self {
    Self {
      id: peer.id,
      device: peer.device,
      last_seen: peer.last_seen.elapsed(),
    }
  }
}
