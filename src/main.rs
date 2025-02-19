use std::{
  collections::HashMap,
  net::{Ipv4Addr, SocketAddr},
  sync::Arc,
};

use discovery::Peers;
use file_transfer::FileTransfer;
use futures::{SinkExt, StreamExt};
use tokio::{
  net::{TcpListener, UdpSocket},
  sync::RwLock,
};

mod api;
mod device;
mod discovery;
mod file_transfer;
mod packet;
mod serde;

const PORT: u16 = 35435;

const INTERFACE_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

const MULTI_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
const MULTICAST_ADDR: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(MULTI_ADDR), PORT);

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let peers = Arc::new(RwLock::new(HashMap::new()));

  let socket = UdpSocket::bind((INTERFACE_ADDR, PORT)).await?;
  let socket = Arc::new(socket);
  socket.join_multicast_v4(MULTI_ADDR, INTERFACE_ADDR)?;
  socket.set_multicast_loop_v4(false)?;

  let listener = TcpListener::bind((INTERFACE_ADDR, PORT)).await?;

  let node_id = uuid::Uuid::new_v4();

  let announcement = packet::Announcement {
    id: node_id,
    device: device::DEVICE,
  };

  let discovery = discovery::Discovery::init(socket, announcement, Arc::clone(&peers)).await?;

  let (_ft, ft_evt_emitter) = FileTransfer::init(Arc::clone(&peers), listener);

  if let Err(e) = discovery.block().await {
    eprintln!("Error: {e}");
  }

  let http_listener = TcpListener::bind("127.0.0.1:35436").await?;
  let app = axum::Router::new()
    .route("/ws", axum::routing::any(ws_handler))
    .with_state(api::AppState::new(&peers, ft_evt_emitter));

  axum::serve(http_listener, app).await
}

async fn ws_handler(
  ws: axum::extract::ws::WebSocketUpgrade,
  state: axum::extract::State<api::AppState>,
  axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<SocketAddr>,
) -> impl axum::response::IntoResponse {
  ws.on_upgrade(move |socket| handle_ws(socket, addr, state))
}

async fn handle_ws(
  socket: axum::extract::ws::WebSocket,
  who: SocketAddr,
  state: axum::extract::State<api::AppState>,
) {
  let (mut sender, mut receiver) = socket.split();

  let send_task = tokio::spawn(async move { loop {} });

  let recv_task = tokio::spawn(async move {
    loop {
      let x = match receiver.next().await {
        Some(value) => value,
        None => continue,
      };

      match x {
        Ok(msg) => {
          todo!()
        }
        Err(e) => eprintln!("{e}"),
      }
    }
  });
}
