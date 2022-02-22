use async_trait::async_trait;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{broadcast, mpsc};
use tokio::{io, signal};

pub struct Shutdown<'a> {
    sender: broadcast::Sender<()>,
    waiter: mpsc::Receiver<()>,
    sender_waiter: mpsc::Sender<()>,
    interrupter: Box<dyn Interrupter + 'a>,
}

pub struct Signaler {
    is_shutdown: bool,
    receiver: broadcast::Receiver<()>,

    #[allow(dead_code)]
    sender_waiter: mpsc::Sender<()>,
}

#[async_trait]
pub trait Interrupter {
    async fn wait(&self) -> io::Result<()>;
}

pub struct CtrlInterrupter {}

impl CtrlInterrupter {
    pub fn new() -> CtrlInterrupter {
        CtrlInterrupter {}
    }
}

#[async_trait]
impl Interrupter for CtrlInterrupter {
    async fn wait(&self) -> io::Result<()> {
        signal::ctrl_c().await
    }
}

impl<'a> Shutdown<'a> {
    pub fn new(interrupter: Box<impl Interrupter + 'a>) -> Shutdown<'a> {
        let (send, recv) = mpsc::channel::<()>(1);
        let (tx, _) = broadcast::channel::<()>(32);
        Shutdown {
            sender: tx,
            waiter: recv,
            sender_waiter: send,
            interrupter,
        }
    }

    pub fn get_signaler(&self) -> Signaler {
        // clone sender_waiter - when all clones go out of scope, waiter.recv() will fire
        Signaler::new(self.sender.subscribe(), self.sender_waiter.clone())
    }

    pub async fn register_shutdown(mut self) -> () {
        self.interrupter
            .wait()
            .await
            .expect("Error waiting for interrupt");

        // send shutdown signal
        println!("waiting for shutdown...");
        self.sender.send(()).expect("Error sending");

        // wait for tasks to finish
        drop(self.sender_waiter);
        let _ = self.waiter.recv().await;
        println!("shutdown");
    }
}

impl Signaler {
    pub fn new(receiver: Receiver<()>, sender_waiter: mpsc::Sender<()>) -> Signaler {
        Signaler {
            is_shutdown: false,
            receiver,
            sender_waiter,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub async fn recv(&mut self) -> io::Result<()> {
        if self.is_shutdown {
            return Ok(());
        }

        let _ = self.receiver.recv().await;
        self.is_shutdown = true;
        Ok(())
    }
}
