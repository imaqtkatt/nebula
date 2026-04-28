use std::sync::Arc;

use tokio::net::UdpSocket;

use nebula_commons::serialization::Deserialize;

#[derive(Clone, Copy)]
pub(crate) struct PeerUpsertedEvent {
    peer: nebula_commons::Peer,
}

impl nebula_event::Event for PeerUpsertedEvent {}

pub(crate) fn discover(
    socket: Arc<UdpSocket>,
    peers: nebula_commons::PeerMap,
    emitter: nebula_event::BoundedEventEmitter,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buf = [0u8; 24];

        loop {
            let (n, addr) = match socket.recv_from(&mut buf).await {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("failed to recv from: {e}");
                    continue;
                }
            };

            let mut buf_reader = tokio::io::BufReader::new(&buf[..n]);
            match nebula_commons::AnnouncePacket::deserialize(&mut buf_reader).await {
                Ok(announcement) => {
                    upsert_peer(Arc::clone(&peers), announcement, addr, &emitter).await;
                }
                Err(e) => {
                    eprintln!("failed to deserialize announce packet: {e}");
                }
            }
        }
    })
}

async fn upsert_peer(
    peers: nebula_commons::PeerMap,
    announcement: nebula_commons::AnnouncePacket,
    addr: std::net::SocketAddr,
    emitter: &nebula_event::BoundedEventEmitter,
) {
    use std::collections::hash_map::Entry;

    let mut peers = peers.write().await;

    let peer = match peers.entry(announcement.id) {
        Entry::Occupied(occupied_entry) => {
            let peer = occupied_entry.into_mut();
            peer.last_seen = tokio::time::Instant::now();
            peer
        }
        Entry::Vacant(vacant_entry) => vacant_entry.insert(nebula_commons::Peer::new(
            announcement.id,
            announcement.device,
            addr,
        )),
    };

    println!("peer upserted? {peer:?}");

    if let Err(e) = emitter.emit(PeerUpsertedEvent { peer: *peer }).await {
        eprintln!("error while emitting PeerUpsertedEvent: {e}");
    }
}

impl nebula_commons::serialization::SerializablePacket for PeerUpsertedEvent {
    const TAG: u8 = 0x1;
}

impl nebula_commons::serialization::Serialize for PeerUpsertedEvent {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        self.peer.serialize(wr).await
    }
}
