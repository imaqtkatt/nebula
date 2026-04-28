mod daemon;
mod daemon_command;

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
const DAEMON_ADDR_V6: Ipv6Addr = Ipv6Addr::LOCALHOST;
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

    let Addrs {
        daemon_socket_addr,
        daemon_addr,
        local_addr,
        multicast_addr,
    } = get_addrs(&cli);

    let (emitter, mut listener) = nebula_event::bounded_event_system(256);

    let this_node_id = nebula_commons::PeerId::new();
    let peers = Arc::new(RwLock::new(HashMap::new()));

    let daemon_socket = UdpSocket::bind(daemon_socket_addr).await?;
    daemon_socket.connect(daemon_addr).await?;
    let daemon_socket = Arc::new(daemon_socket);

    nebula_discovery::Discovery::configure_events(&mut listener, &daemon_socket);

    let discovery_socket = UdpSocket::bind(local_addr).await?;
    let discovery_socket = Arc::new(discovery_socket);
    if cli.ipv6 {
        discovery_socket.join_multicast_v6(&MULTI_ADDR_V6, 0)?;
        discovery_socket.set_multicast_loop_v6(false)?;
    } else {
        discovery_socket.join_multicast_v4(MULTI_ADDR_V4, INTERFACE_ADDR_V4)?;
        discovery_socket.set_multicast_loop_v4(false)?;
    }

    let _discovery = nebula_discovery::Discovery::init(
        discovery_socket,
        multicast_addr,
        this_node_id,
        std::sync::Arc::clone(&peers),
        emitter,
    );

    let _daemon = crate::daemon::init(daemon_socket, peers);

    listener.run().await;

    panic!("listener closed");
}

struct Addrs {
    daemon_socket_addr: SocketAddr,
    daemon_addr: SocketAddr,
    local_addr: SocketAddr,
    multicast_addr: SocketAddr,
}

fn get_addrs(cli: &Cli) -> Addrs {
    Addrs {
        daemon_socket_addr: if cli.ipv6 {
            SocketAddr::V6(SocketAddrV6::new(DAEMON_ADDR_V6, 0, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(DAEMON_ADDR_V4, 0))
        },
        daemon_addr: if cli.ipv6 {
            SocketAddr::V6(SocketAddrV6::new(DAEMON_ADDR_V6, DAEMON_PORT, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(DAEMON_ADDR_V4, DAEMON_PORT))
        },
        local_addr: if cli.ipv6 {
            SocketAddr::V6(SocketAddrV6::new(INTERFACE_ADDR_V6, PORT, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(INTERFACE_ADDR_V4, PORT))
        },
        multicast_addr: if cli.ipv6 {
            MULTICAST_ADDR_V6
        } else {
            MULTICAST_ADDR_V4
        },
    }
}
