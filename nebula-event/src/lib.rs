use tokio::sync::mpsc;

/// Marker trait to constrain what types may be emitted.
pub trait Event: Clone + Send + 'static {}

type Handler =
    Box<dyn Fn(&dyn std::any::Any) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Event listener implemented by a bounded tokio channel.
pub struct BoundedEventListener {
    handlers: std::collections::HashMap<std::any::TypeId, Vec<Handler>>,
    rx: mpsc::Receiver<Box<dyn std::any::Any + Send>>,
}

/// Event emitter for a [`BoundedEventListener`].
///
/// Can be cloned across threads.
#[derive(Clone)]
pub struct BoundedEventEmitter {
    tx: mpsc::Sender<Box<dyn std::any::Any + Send>>,
}

impl BoundedEventListener {
    pub fn register<E, F, Fut>(&mut self, handler: F)
    where
        E: Event,
        Fut: Future<Output = ()> + Send + 'static,
        F: Fn(E) -> Fut + Send + 'static,
    {
        let type_id = std::any::TypeId::of::<E>();
        let handlers = self.handlers.entry(type_id).or_default();

        handlers.push(Box::new(move |e| {
            let e = e.downcast_ref().cloned().unwrap();
            Box::pin(handler(e))
        }));
    }

    pub async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            let type_id = event.as_ref().type_id();

            if let Some(handlers) = self.handlers.get_mut(&type_id) {
                for handler in handlers {
                    handler(event.as_ref()).await;
                }
            }
        }
    }
}

impl BoundedEventEmitter {
    pub async fn emit<E: Event>(
        &self,
        event: E,
    ) -> Result<(), mpsc::error::SendError<Box<dyn std::any::Any + Send + 'static>>> {
        self.tx.send(Box::new(event)).await
    }
}

pub fn bounded_event_system(buffer: usize) -> (BoundedEventEmitter, BoundedEventListener) {
    let (tx, rx) = mpsc::channel(buffer);

    let emitter = BoundedEventEmitter { tx };

    let listener = BoundedEventListener {
        handlers: Default::default(),
        rx,
    };

    (emitter, listener)
}
