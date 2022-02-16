use async_trait::async_trait;
use tokio::sync::broadcast;

#[async_trait]
pub trait Stopper {
    //fn new(notify: broadcast::Receiver<()>) -> Self;
    fn is_shutdown(&self) -> bool;
    async fn recv(&mut self);
}

pub struct Shutdown {
    /// `true` if the shutdown signal has been received
    pub shutdown: bool,
    /// The receive half of the channel used to listen for shutdown.
    pub notify: broadcast::Receiver<()>,
}

//#[async_trait]
impl Shutdown {
    // fn new(notify: broadcast::Receiver<()>) -> Shutdown {
    //     Shutdown {
    //         shutdown: false,
    //         notify,
    //     }
    // }

    /// Returns `true` if the shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.shutdown = true;
    }
}
