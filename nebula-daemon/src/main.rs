use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
};

use clap::Parser;
use tokio::{net::UdpSocket, sync::RwLock};

const PORT: u16 = 35435;

const INTERFACE_ADDR_V4: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const INTERFACE_ADDR_V6: Ipv6Addr = Ipv6Addr::UNSPECIFIED;

const MULTI_ADDR_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const MULTICAST_ADDR_V4: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(MULTI_ADDR_V4), PORT);

const MULTI_ADDR_V6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
const MULTICAST_ADDR_V6: SocketAddr = SocketAddr::new(std::net::IpAddr::V6(MULTI_ADDR_V6), PORT);

#[derive(Debug, clap::Parser)]
struct Cli {
    #[arg(long)]
    ipv6: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let local_addr = if cli.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(INTERFACE_ADDR_V6, PORT, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(INTERFACE_ADDR_V4, PORT))
    };

    let node_id = nebula_commons::PeerId::new();
    let peers = Arc::new(RwLock::new(HashMap::new()));

    let socket = UdpSocket::bind(local_addr).await?;
    let socket = Arc::new(socket);

    let _discovery = nebula_discovery::Discovery::init(socket, MULTICAST_ADDR_V4, node_id, peers);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
