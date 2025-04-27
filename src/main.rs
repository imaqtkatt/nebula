use std::{
  collections::HashMap,
  net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
  sync::Arc,
};

use file_transfer::{FileTransfer, FileTransferWsEvent};
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
mod header;
mod packet;
mod serde;

const PORT: u16 = 35435;

const INTERFACE_ADDR_V4: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const INTERFACE_ADDR_V6: Ipv6Addr = Ipv6Addr::UNSPECIFIED;

const MULTI_ADDR_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const MULTICAST_ADDR_V4: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(MULTI_ADDR_V4), PORT);

const MULTI_ADDR_V6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
const MULTICAST_ADDR_V6: SocketAddr = SocketAddr::new(std::net::IpAddr::V6(MULTI_ADDR_V6), PORT);

const USE_IPV6: bool = false;

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let peers = Arc::new(RwLock::new(HashMap::new()));

  let socket = if USE_IPV6 {
    let local_addr = SocketAddrV6::new(INTERFACE_ADDR_V6, PORT, 0, 0);
    UdpSocket::bind(local_addr).await?
  } else {
    let local_addr = SocketAddrV4::new(INTERFACE_ADDR_V4, PORT);
    UdpSocket::bind(local_addr).await?
  };

  let socket = Arc::new(socket);

  if USE_IPV6 {
    socket.join_multicast_v6(&MULTI_ADDR_V6, 0)?;
    socket.set_multicast_loop_v6(false)?;
  } else {
    socket.join_multicast_v4(MULTI_ADDR_V4, INTERFACE_ADDR_V4)?;
    socket.set_multicast_loop_v4(false)?;
  }

  let listener = if USE_IPV6 {
    TcpListener::bind((INTERFACE_ADDR_V6, PORT)).await?
  } else {
    TcpListener::bind((INTERFACE_ADDR_V4, PORT)).await?
  };

  let node_id = uuid::Uuid::new_v4();

  let announcement = packet::Announcement {
    id: node_id,
    device: device::DEVICE,
  };

  let (event_sender, event_receiver) = event::new_channel();
  discovery::Discovery::<USE_IPV6>::init(socket, announcement, &event_sender, Arc::clone(&peers))
    .await?;

  let (_file_transfer, file_transfer_emitter) = FileTransfer::init(Arc::clone(&peers), listener);

  let state = api::AppState::new(&peers, file_transfer_emitter, event_receiver);

  let http_listener = TcpListener::bind("localhost:35436").await?;
  let app = axum::Router::new()
    .route("/ws", axum::routing::any(ws_handler))
    .route("/page", axum::routing::get(page))
    .with_state(state);

  println!("page available @ http://localhost:35436/page");

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
) -> impl axum::response::IntoResponse {
  ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(
  socket: axum::extract::ws::WebSocket,
  axum::extract::State(state): axum::extract::State<api::AppState>,
) {
  let (mut sender, mut receiver) = socket.split();

  let event_receiver = state.event_receiver.clone();

  let mut send_task = tokio::spawn(async move {
    let mut receiver = event_receiver.lock().await;

    loop {
      if let Some(event) = receiver.recv().await {
        let json_message = match serde_json::to_string(&event) {
          Ok(value) => value,
          Err(e) => {
            eprintln!("Error while serializing event: {e}");
            continue;
          }
        };

        if let Err(e) = sender
          .send(axum::extract::ws::Message::Text(json_message.into()))
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
        eprintln!("WS receive error: {e}");
        continue;
      }

      if let Ok(axum::extract::ws::Message::Close(_)) = msg {
        break;
      }

      if let Ok(axum::extract::ws::Message::Text(text)) = msg {
        let value = match serde_json::from_str::<FileTransferWsEvent>(text.as_str()) {
          Ok(value) => value,
          Err(e) => {
            eprintln!("Error while deserializing file transfer event: {e}");
            continue;
          }
        };

        if let Err(e) = state.emit_file_transfer_event(value).await {
          eprintln!("Error while sending file transfer event: {e}");
        }
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
