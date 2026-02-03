use std::sync::Arc;

use tokio::net::UdpSocket;

use nebula_commons::serialization::Deserialize;

pub(crate) fn discover(
    socket: Arc<UdpSocket>,
    peers: nebula_commons::PeerMap,
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

            {
                let mut buf = tokio::io::BufReader::new(&buf[..n]);
                let announcement = match nebula_commons::AnnouncePacket::deserialize(&mut buf).await
                {
                    Ok(value) => value,
                    Err(e) => {
                        eprintln!("failed to deserialize announce packet: {e}");
                        continue;
                    }
                };

                upsert_peer(Arc::clone(&peers), announcement, addr).await;
            }

            buf = [0u8; 24];
        }
    })
}

async fn upsert_peer(
    peers: nebula_commons::PeerMap,
    announcement: nebula_commons::AnnouncePacket,
    addr: std::net::SocketAddr,
) {
    use std::collections::hash_map::Entry;

    let mut peers = peers.write().await;

    let _peer = match peers.entry(announcement.id) {
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
}
