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
        emitter: nebula_event::BoundedEventEmitter,
    ) -> Self {
        let announce_packet = nebula_commons::AnnouncePacket {
            id: node_id,
            device: nebula_commons::CURRENT_DEVICE,
        };

        Self {
            discover_task: task::discover::discover(Arc::clone(&socket), Arc::clone(&peers)),
            accept_task: task::announce::announce(
                Arc::clone(&socket),
                multi_addr,
                announce_packet,
                emitter.clone(),
            ),
            remove_task: task::remove::remove(peers, emitter),
        }
    }

    pub fn configure_events(
        listener: &mut nebula_event::BoundedEventListener,
        daemon_socket: std::sync::Arc<tokio::net::UdpSocket>,
    ) {
        let daemon_socket = daemon_socket.clone();
        let daemon_socket2 = daemon_socket.clone();

        listener
            .register(move |e| tell_peer_disconnected(std::sync::Arc::clone(&daemon_socket), e));

        listener.register(move |e| tell_announced(std::sync::Arc::clone(&daemon_socket2), e));
    }
}

async fn tell_peer_disconnected(
    daemon: std::sync::Arc<tokio::net::UdpSocket>,
    _: crate::task::remove::PeerDisconnectedEvent,
) {
    _ = daemon.send(b"Hello").await;
}

async fn tell_announced(
    daemon: std::sync::Arc<tokio::net::UdpSocket>,
    _: crate::task::announce::Announced,
) {
    if let Err(e) = daemon.send(b"Ok").await {
        panic!("DUMB: {e}");
    }
    println!("announced");
}
