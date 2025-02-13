pub trait Serialize: Sized {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(&self, wr: &mut W)
    -> std::io::Result<()>;
}

pub trait Deserialize: Sized {
  async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self>;
}
