use std::{
  collections::HashMap,
  net::{Ipv4Addr, SocketAddr},
  sync::Arc,
};

use axum::ServiceExt;
use file_transfer::FileTransfer;
use futures::{SinkExt, StreamExt};
use tokio::{
  net::{TcpListener, UdpSocket},
  sync::RwLock,
};

mod api;
mod device;
mod discovery;
mod event;
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

  let (event_sender, event_receiver) = event::new_channel();
  let _discovery =
    discovery::Discovery::init(socket, announcement, &event_sender, Arc::clone(&peers)).await?;

  let (_file_transfer, file_transfer_emitter) = FileTransfer::init(Arc::clone(&peers), listener);

  let state = api::AppState::new(&peers, file_transfer_emitter, event_receiver);

  let http_listener = TcpListener::bind("localhost:35436").await?;
  let app = axum::Router::new()
    .route("/ws", axum::routing::any(ws_handler))
    .route("/page", axum::routing::get(page))
    .with_state(state);

  axum::serve(
    http_listener,
    app.into_make_service_with_connect_info::<SocketAddr>(),
  )
  .await
}

async fn page() -> impl axum::response::IntoResponse {
  const PAGE: &[u8] = include_bytes!("../assets/page.html");

  axum::response::Html(PAGE)
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
  _who: SocketAddr,
  axum::extract::State(state): axum::extract::State<api::AppState>,
) {
  let (mut sender, mut receiver) = socket.split();

  let mut send_task = tokio::spawn(async move {
    let mut receiver = state.event_receiver.lock().await;

    loop {
      if let Some(event) = receiver.recv().await {
        let json_message = match serde_json::to_vec(&event) {
          Ok(value) => value,
          Err(e) => {
            eprintln!("Error while serializing event: {e}");
            continue;
          }
        };

        if let Err(e) = sender
          .send(axum::extract::ws::Message::Binary(json_message.into()))
          .await
        {
          eprintln!("Error while sending event: {e}");
        }
      }
    }
  });

  let mut recv_task = tokio::spawn(async move {
    loop {
      let msg = match receiver.next().await {
        Some(value) => value,
        None => continue,
      };

      if let Err(e) = msg {
        eprintln!("{e}");
        continue;
      }

      if let Ok(axum::extract::ws::Message::Close(x)) = msg {
        break;
      }

      if let Ok(axum::extract::ws::Message::Text(x)) = msg {
        todo!("{x}")
      }
    }
  });

  tokio::select! {
    _rv_a = (&mut send_task) => {
      recv_task.abort();
    }
    _rv_b = (&mut recv_task) => {
      send_task.abort();
    }
  }
}
