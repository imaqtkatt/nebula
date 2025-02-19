use std::time::Duration;

use crate::{discovery::Peers, file_transfer};

#[derive(Clone)]
pub struct AppState {
  peers: Peers,
  file_transfer_event_emitter: tokio::sync::mpsc::UnboundedSender<file_transfer::TransferData>,
}

impl AppState {
  pub fn new(
    peers: &Peers,
    file_transfer_event_emitter: tokio::sync::mpsc::UnboundedSender<file_transfer::TransferData>,
  ) -> Self {
    Self {
      peers: std::sync::Arc::clone(peers),
      file_transfer_event_emitter,
    }
  }
}

#[derive(serde::Serialize)]
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

pub async fn get_peers(state: axum::extract::State<AppState>) -> axum::Json<Vec<JsonPeer>> {
  let peers_read = state.peers.read().await;

  let json_peers = peers_read
    .values()
    .copied()
    .map(JsonPeer::from)
    .collect::<Vec<_>>();

  axum::Json(json_peers)
}
