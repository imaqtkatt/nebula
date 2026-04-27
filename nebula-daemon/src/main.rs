mod daemon;

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

const DAEMON_ADDR_V4: Ipv4Addr = Ipv4Addr::LOCALHOST;
const DAEMON_ADDR_V6: Ipv4Addr = Ipv4Addr::LOCALHOST;
const DAEMON_PORT: u16 = 35436;

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

    // println!("using ipv6 = {}", cli.ipv6);

    let daemon_addr = if cli.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, DAEMON_PORT, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DAEMON_PORT))
    };

    let local_addr = if cli.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(INTERFACE_ADDR_V6, PORT, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(INTERFACE_ADDR_V4, PORT))
    };

    let (emitter, mut listener) = nebula_event::bounded_event_system(256);

    let this_node_id = nebula_commons::PeerId::new();
    let peers = Arc::new(RwLock::new(HashMap::new()));

    let daemon_socket = UdpSocket::bind("127.0.0.1:0").await?;
    daemon_socket.connect(daemon_addr).await?;
    let daemon_socket = Arc::new(daemon_socket);

    nebula_discovery::Discovery::configure_events(&mut listener, Arc::clone(&daemon_socket));

    let discovery_socket = UdpSocket::bind(local_addr).await?;
    let discovery_socket = Arc::new(discovery_socket);

    let _discovery = nebula_discovery::Discovery::init(
        discovery_socket,
        MULTICAST_ADDR_V4,
        this_node_id,
        peers,
        emitter,
    );

    let mut buf = [0; 256];
    tokio::spawn(async move {
        loop {
            let size = match daemon_socket.recv(&mut buf).await {
                Ok(value) => value,
                Err(_) => continue,
            };

            println!("received: {:?}", &buf[..size]);
        }
    });

    listener.run().await;

    panic!("listener closed");
}
