use std::time;

const REMOVE_SLEEP_SECS: time::Duration = time::Duration::from_secs(10);
const PEER_TIMEOUT_SECS: time::Duration = time::Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
pub(crate) struct PeerDisconnectedEvent {
    pub peer_id: nebula_commons::PeerId,
}

impl nebula_event::Event for PeerDisconnectedEvent {}

pub(crate) fn remove(
    peers: nebula_commons::PeerMap,
    emitter: nebula_event::BoundedEventEmitter,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(REMOVE_SLEEP_SECS).await;

            let mut peers_write = peers.write().await;

            let peers_to_remove: Vec<_> = peers_write
                .iter()
                .filter(|(_, peer)| peer.last_seen.elapsed() > PEER_TIMEOUT_SECS)
                .map(|(id, _)| *id)
                .collect();

            for peer_id in peers_to_remove {
                if let Err(e) = emitter.emit(PeerDisconnectedEvent { peer_id }).await {
                    eprintln!("error while emitting PeerDisconnectedEvent: {e}")
                }
                peers_write.remove(&peer_id);
            }
        }
    })
}

impl nebula_commons::serialization::SerializablePacket for PeerDisconnectedEvent {
    const TAG: u8 = 0x2;
}

impl nebula_commons::serialization::Serialize for PeerDisconnectedEvent {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        self.peer_id.serialize(wr).await
    }
}
