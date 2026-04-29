use nebula_commons::serialization::Deserialize;

#[derive(Clone, Debug)]
pub enum DaemonCommand {
    RefreshPeers,
    SendFileTo(SendFileToCommand),
}

#[derive(Clone, Debug)]
pub struct SendFileToCommand {
    pub peer_id: nebula_commons::PeerId,
    pub rand_id: u16,
    pub file_path: String,
}

impl Deserialize for DaemonCommand {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let tag = rd.read_u8().await?;

        match tag {
            0x01 => Ok(Self::RefreshPeers),
            0x02 => SendFileToCommand::deserialize(rd)
                .await
                .map(Self::SendFileTo),
            _ => Err(std::io::Error::other("bad command tag")),
        }
    }
}

impl Deserialize for SendFileToCommand {
    async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
        let peer_id = nebula_commons::PeerId::deserialize(rd).await?;
        let rand_id = rd.read_u16().await?;

        let file_path_len = rd.read_u16().await?;
        let mut file_path_buf = vec![0; file_path_len as usize];
        _ = rd.read_exact(&mut file_path_buf).await?;

        let file_path = String::from_utf8(file_path_buf).map_err(std::io::Error::other)?;

        Ok(Self {
            peer_id,
            rand_id,
            file_path,
        })
    }
}
