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
