use std::{io::Write, net::SocketAddr, path::PathBuf};

use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
};

use crate::discovery::Peers;

#[derive(Debug)]
pub struct TransferData {
  addr: SocketAddr,
  data: Vec<u8>,
}

pub struct FileTransfer {
  peers: Peers,
  sender: Option<tokio::task::JoinHandle<()>>,
  receiver: Option<tokio::task::JoinHandle<()>>,
}

impl FileTransfer {
  pub fn init(
    peers: Peers,
    listener: TcpListener,
  ) -> (Self, tokio::sync::mpsc::UnboundedSender<TransferData>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let this = Self {
      peers,
      sender: Some(sender(rx)),
      receiver: Some(receiver(listener)),
    };

    (this, tx)
  }
}

fn sender(
  mut evt_listener: tokio::sync::mpsc::UnboundedReceiver<TransferData>,
) -> tokio::task::JoinHandle<()> {
  tokio::task::spawn(async move {
    loop {
      if let Some(evt) = evt_listener.recv().await {
        let mut stream = match tokio::net::TcpStream::connect(evt.addr).await {
          Ok(value) => value,
          Err(e) => {
            eprintln!("Error while connecting to peer: {e}");
            continue;
          }
        };

        if let Err(e) = stream.write_all(&evt.data).await {
          eprintln!("Error while sending file data: {e}");
        }
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
  const MAX_BYTES: usize = 33554432;

  let mut read = 0;
  let mut buf = [0u8; 1024];

  let file_name = format!("{}_nebula", std::time::Instant::now().elapsed().as_secs());
  let mut file_path = PathBuf::new();
  file_path.push(file_name);
  file_path.set_extension("tmp");

  let mut file = std::fs::File::options()
    .create(true)
    .write(true)
    .open(file_path)?;

  loop {
    if read > MAX_BYTES {
      todo!("");
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
