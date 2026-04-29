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
            discover_task: task::discover::discover(
                Arc::clone(&socket),
                Arc::clone(&peers),
                emitter.clone(),
            ),
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
        daemon_socket: &std::sync::Arc<tokio::net::UdpSocket>,
    ) {
        let daemon_socket = daemon_socket.clone();
        let daemon_socket2 = daemon_socket.clone();

        listener.register(move |e| {
            let daemon_socket = daemon_socket.clone();
            async move {
                if let Err(e) = tell_peer_disconnected(daemon_socket, e).await {
                    eprintln!("{e:?}")
                }
            }
        });

        listener.register(move |e| {
            let daemon_socket = daemon_socket2.clone();
            async move {
                if let Err(e) = tell_peer_upserted(daemon_socket, e).await {
                    eprintln!("{e}")
                }
            }
        });
    }
}

async fn tell_peer_disconnected(
    daemon: std::sync::Arc<tokio::net::UdpSocket>,
    e: crate::task::remove::PeerDisconnectedEvent,
) -> std::io::Result<()> {
    use nebula_commons::serialization::SerializablePacket;

    let mut buf_writer = tokio::io::BufWriter::new(vec![]);
    e.serialize(&mut buf_writer).await?;

    daemon.send(buf_writer.buffer()).await?;

    Ok(())
}

async fn tell_peer_upserted(
    daemon: std::sync::Arc<tokio::net::UdpSocket>,
    e: crate::task::discover::PeerUpsertedEvent,
) -> std::io::Result<()> {
    use nebula_commons::serialization::SerializablePacket;

    let mut buf_writer = tokio::io::BufWriter::new(vec![]);
    e.serialize(&mut buf_writer).await?;

    daemon.send(buf_writer.buffer()).await?;

    Ok(())
}
