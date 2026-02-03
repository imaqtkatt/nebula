use std::time;

const REMOVE_SLEEP_SECS: time::Duration = time::Duration::from_secs(10);
const PEER_TIMEOUT_SECS: time::Duration = time::Duration::from_secs(30);

pub(crate) fn remove(peers: nebula_commons::PeerMap) -> tokio::task::JoinHandle<()> {
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
                peers_write.remove(&peer_id);
            }
        }
    })
}
