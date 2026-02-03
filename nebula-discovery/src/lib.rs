use std::sync::Arc;

use tokio::net::UdpSocket;

mod task;

pub struct Discovery {
    pub discover_task: tokio::task::JoinHandle<()>,
    pub accept_task: tokio::task::JoinHandle<()>,
    pub remove_task: tokio::task::JoinHandle<()>,
}

impl Discovery {
    pub fn init(
        socket: Arc<UdpSocket>,
        multi_addr: core::net::SocketAddr,
        node_id: nebula_commons::PeerId,
        peers: nebula_commons::PeerMap,
    ) -> Self {
        let announce_packet = nebula_commons::AnnouncePacket {
            id: node_id,
            device: nebula_commons::CURRENT_DEVICE,
        };

        Self {
            discover_task: task::discover::discover(Arc::clone(&socket), Arc::clone(&peers)),
            accept_task: task::announce::announce(Arc::clone(&socket), multi_addr, announce_packet),
            remove_task: task::remove::remove(peers),
        }
    }
}
