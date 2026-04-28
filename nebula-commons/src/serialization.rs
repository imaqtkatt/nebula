use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait Serialize: Sized {
    fn serialize<W: AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> impl std::future::Future<Output = std::io::Result<()>>;
}

pub trait Deserialize: Sized {
    fn deserialize<R: AsyncReadExt + Unpin>(
        rd: &mut R,
    ) -> impl std::future::Future<Output = std::io::Result<Self>>;
}

pub trait SerializablePacket: Serialize {
    const TAG: u8;

    fn serialize<W: AsyncWriteExt + Unpin>(
        &self,
        wr: &mut W,
    ) -> impl std::future::Future<Output = std::io::Result<()>> {
        async {
            wr.write_u8(Self::TAG).await?;
            <Self as Serialize>::serialize(self, wr).await
        }
    }
}
