use nebula_commons::serialization::{self, Deserialize};
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

            if let Err(e) = crate::daemon::run_daemon_command(
                command,
                std::sync::Arc::clone(&daemon_socket),
                &peers,
            )
            .await
            {
                eprintln!("error while running command: {e}");
            }
        }
    })
}

async fn run_daemon_command(
    daemon_command: DaemonCommand,
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: &nebula_commons::PeerMap,
) -> std::io::Result<()> {
    match daemon_command {
        DaemonCommand::SendFileTo(command) => run_send_file_to(command, daemon_socket, peers).await,
        DaemonCommand::RefreshPeers => run_refresh_peers(daemon_socket, peers).await,
    }
}

struct FileSendingSuccess {
    rand_id: u16,
}

struct FileSendingError {
    rand_id: u16,
}

async fn run_send_file_to(
    send_file_to_command: SendFileToCommand,
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: &nebula_commons::PeerMap,
) -> std::io::Result<()> {
    let peers = peers.blocking_read();
    if let Some(peer) = peers.get(&send_file_to_command.peer_id) {
        println!("{:?}, {:?}, {:?}", peer.id, peer.addr, peer.device);
    }
    _ = daemon_socket;

    Ok(())
}

struct RefreshPeers<'a> {
    peers: Vec<&'a nebula_commons::Peer>,
}

async fn run_refresh_peers(
    daemon_socket: std::sync::Arc<UdpSocket>,
    peers: &nebula_commons::PeerMap,
) -> std::io::Result<()> {
    let peers = peers.blocking_read();
    let peers = peers.values().collect();

    let refresh_peers = RefreshPeers { peers };

    use serialization::SerializablePacket;
    let mut buf = tokio::io::BufWriter::new(vec![]);
    refresh_peers.serialize(&mut buf).await?;

    let size = daemon_socket.send(buf.buffer()).await?;

    assert!(size == buf.buffer().len());

    Ok(())
}

impl serialization::Serialize for FileSendingSuccess {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        wr.write_u16(self.rand_id).await?;
        Ok(())
    }
}

impl serialization::SerializablePacket for FileSendingSuccess {
    const TAG: u8 = 0x3;
}

impl serialization::Serialize for FileSendingError {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        wr.write_u16(self.rand_id).await?;
        Ok(())
    }
}

impl serialization::SerializablePacket for FileSendingError {
    const TAG: u8 = 0x4;
}

impl serialization::Serialize for RefreshPeers<'_> {
    async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> std::io::Result<()> {
        let peers_len = self.peers.len() as u32;
        wr.write_u32(peers_len).await?;

        for peer in self.peers.iter() {
            peer.serialize(wr).await?;
        }

        Ok(())
    }
}

impl serialization::SerializablePacket for RefreshPeers<'_> {
    const TAG: u8 = 0x5;
}
