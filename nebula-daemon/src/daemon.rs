// pub struct Daemon {
//     tcp_listener: tokio::net::TcpListener,
//     event_listener: nebula_event::BoundedEventListener,
// }

// impl Daemon {
//     pub async fn run(mut self) -> std::io::Result<()> {
//         let (stream, _) = self.tcp_listener.accept().await?;

//         // self.event_listener.register(|a: | todo!());
//         // self.event_listener.run().await;

//         Ok(())
//     }
// }
