use crate::api;

#[derive(Debug, serde::Serialize)]
pub enum NebulaEvent {
  PeerUpserted { peer: api::JsonPeer },
  PeerDisconnected { id: uuid::Uuid },
}

pub type EventSender = tokio::sync::mpsc::UnboundedSender<NebulaEvent>;

pub type EventReceiver = tokio::sync::mpsc::UnboundedReceiver<NebulaEvent>;

pub fn new_channel() -> (EventSender, EventReceiver) {
  tokio::sync::mpsc::unbounded_channel()
}
