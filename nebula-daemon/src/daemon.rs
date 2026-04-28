use nebula_commons::serialization::Deserialize;
use tokio::net::UdpSocket;

use crate::daemon_command::{self, DaemonCommand, SendFileToCommand};

pub fn init(
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: nebula_commons::PeerMap,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buf = [0; 256];

        loop {
            let size = match daemon_socket.recv(&mut buf).await {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("error while recv: {e}");
                    continue;
                }
            };

            let mut buf = tokio::io::BufReader::new(&buf[..size]);
            let command = match daemon_command::DaemonCommand::deserialize(&mut buf).await {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("error while deserializing DaemonCommand: {e}");
                    continue;
                }
            };

            crate::daemon::run_daemon_command(
                command,
                std::sync::Arc::clone(&daemon_socket),
                &peers,
            );
        }
    })
}

pub fn run_daemon_command(
    daemon_command: DaemonCommand,
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: &nebula_commons::PeerMap,
) {
    match daemon_command {
        DaemonCommand::SendFileTo(command) => run_send_file_to(command, daemon_socket, peers),
        DaemonCommand::RefreshPeers => run_refresh_peers(daemon_socket, peers),
    }
}

fn run_send_file_to(
    send_file_to_command: SendFileToCommand,
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: &nebula_commons::PeerMap,
) {
    let peers = peers.blocking_read();
    if let Some(peer) = peers.get(&send_file_to_command.peer_id) {
        println!("{:?}, {:?}, {:?}", peer.id, peer.addr, peer.device);
    }
    _ = daemon_socket;
}

fn run_refresh_peers(daemon_socket: std::sync::Arc<UdpSocket>, peers: &nebula_commons::PeerMap) {
    _ = daemon_socket;
    _ = peers;
}
