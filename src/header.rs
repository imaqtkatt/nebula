use crate::serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Header {
  pub file_name: String,
  pub file_ext: String,
}

impl Serialize for Header {
  async fn serialize<W: tokio::io::AsyncWriteExt + Unpin>(
    &self,
    wr: &mut W,
  ) -> std::io::Result<()> {
    let file_name_len = self.file_name.len();
    let file_ext_len = self.file_ext.len();
    assert!(self.file_name.len() < u8::MAX as usize);
    assert!(self.file_ext.len() < u8::MAX as usize);

    let file_name_len = file_name_len as u8;
    let file_ext_len = file_ext_len as u8;

    wr.write_u8(file_name_len).await?;
    wr.write_all(self.file_name.as_bytes()).await?;

    wr.write_u8(file_ext_len).await?;
    wr.write_all(self.file_ext.as_bytes()).await?;

    Ok(())
  }
}

impl Deserialize for Header {
  async fn deserialize<R: tokio::io::AsyncReadExt + Unpin>(rd: &mut R) -> std::io::Result<Self> {
    let file_name_len = rd.read_u8().await? as usize;
    let mut file_name = vec![0; file_name_len];
    let len = rd.read(&mut file_name).await?;
    debug_assert_eq!(file_name_len, len);
    let file_name = String::from_utf8(file_name).map_err(|e| std::io::Error::other(e))?;

    let file_ext_len = rd.read_u8().await? as usize;
    let mut file_ext = vec![0; file_ext_len];
    let len = rd.read(&mut file_ext).await?;
    debug_assert_eq!(file_ext_len, len);
    let file_ext = String::from_utf8(file_ext).map_err(|e| std::io::Error::other(e))?;

    Ok(Self {
      file_name,
      file_ext,
    })
  }
}
