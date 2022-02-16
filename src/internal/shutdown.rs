use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::{broadcast, Mutex};

pub struct Shutdown {
    is_shutdown: Mutex<bool>,
    receiver: Mutex<broadcast::Receiver<()>>,
    sender: broadcast::Sender<()>,
}

impl Shutdown {
    pub fn new() -> Shutdown {
        let (tx, rx) = broadcast::channel::<()>(32);
        Shutdown {
            is_shutdown: Mutex::new(false),
            receiver: Mutex::new(rx),
            sender: tx,
        }
    }

    pub fn get_receiver(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    pub fn shutdown(&self) -> Result<usize, SendError<()>> {
        self.sender.send(())
    }

    pub async fn is_shutdown(&self) -> bool {
        self.is_shutdown.lock().await.clone()
    }

    pub fn register(self: Arc<Self>) -> Arc<Self> {
        tokio::spawn({
            let s = Arc::clone(&self);
            async move {
                // wait for receive signal
                let mut me = s.receiver.lock().await;
                let _ = me.recv();

                // update is_shutdown
                let mut s = s.is_shutdown.lock().await;
                *s = true
            }
        });

        self
    }
}
