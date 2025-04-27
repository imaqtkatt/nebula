use std::{io::Write, net::SocketAddr, path::PathBuf};

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
};

use crate::{
  discovery::Peers,
  serde::{Deserialize, Serialize},
};

#[derive(Debug)]
pub struct TransferData {
  pub addr: SocketAddr,
  pub extension: String,
  pub file_name: String,
  pub data: Vec<u8>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename(deserialize = "FileTransfer"))]
pub struct FileTransferWsEvent {
  pub id: uuid::Uuid,
  pub extension: String,
  pub file_name: String,
  pub data: String,
}

pub struct FileTransfer {
  peers: Peers,
  sender: tokio::task::JoinHandle<()>,
  receiver: tokio::task::JoinHandle<()>,
}

impl FileTransfer {
  pub fn init(
    peers: Peers,
    listener: TcpListener,
  ) -> (Self, tokio::sync::mpsc::UnboundedSender<TransferData>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let this = Self {
      peers,
      sender: sender(rx),
      receiver: receiver(listener),
    };

    (this, tx)
  }
}

fn sender(
  mut evt_listener: tokio::sync::mpsc::UnboundedReceiver<TransferData>,
) -> tokio::task::JoinHandle<()> {
  tokio::task::spawn(async move {
    loop {
      if let Some(event) = evt_listener.recv().await {
        let mut stream = match tokio::net::TcpStream::connect(event.addr).await {
          Ok(value) => value,
          Err(e) => {
            eprintln!("Error while connecting to peer: {e}");
            continue;
          }
        };

        tokio::spawn(async move {
          let header = crate::header::Header {
            file_name: event.file_name,
            file_ext: event.extension,
          };
          if let Err(e) = header.serialize(&mut stream).await {
            eprintln!("Error while serializing file header: {e}");
            return;
          }

          if let Err(e) = stream.write_all(&event.data).await {
            eprintln!("Error while sending file data: {e}");
          }
        });
      }
    }
  })
}

fn receiver(listener: TcpListener) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    loop {
      let (stream, _) = match listener.accept().await {
        Ok(value) => value,
        Err(e) => {
          eprintln!("Error while accepting connection: {e}");
          continue;
        }
      };

      tokio::spawn(async move {
        if let Err(e) = receive_file(stream).await {
          eprintln!("Error while receiving file: {e}");
        }
      });
    }
  })
}

async fn receive_file(mut stream: TcpStream) -> std::io::Result<()> {
  let header = crate::header::Header::deserialize(&mut stream).await?;

  const MAX_BYTES: usize = 33554432;

  let mut read = 0;
  let mut buf = [0u8; 1024];

  let time = match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
    Ok(duration) => duration.as_secs().to_string(),
    Err(e) => return Err(std::io::Error::other(e)),
  };
  let file_name = format!("./{}_{}", header.file_name, time);
  let mut file_path = PathBuf::new();
  file_path.push(file_name);
  file_path.set_extension(header.file_ext);

  let mut file = std::fs::File::options()
    .create(true)
    .write(true)
    .open(file_path)?;

  loop {
    if read > MAX_BYTES {
      todo!("MAX_BYTES exceeded");
    }

    let n = stream.read(&mut buf).await?;
    read += n;

    if n == 0 {
      break;
    }

    file.write_all(&buf[..n])?;
  }

  Ok(())
}
