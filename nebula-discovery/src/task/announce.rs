use std::{sync::Arc, time};

use nebula_commons::serialization::Serialize;
use tokio::net::UdpSocket;

pub(crate) fn announce(
    socket: Arc<UdpSocket>,
    multi_addr: core::net::SocketAddr,
    announce_packet: nebula_commons::AnnouncePacket,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buf = vec![];
        if let Err(e) = announce_packet.serialize(&mut buf).await {
            panic!("failed to serialize announce packet: {e}");
        }

        loop {
            if let Err(e) = socket.send_to(&buf, multi_addr).await {
                eprintln!("error while sending announcement: {e}");
            }

            tokio::time::sleep(time::Duration::from_secs(5)).await;
        }
    })
}
