mod shutdown;

use async_trait::async_trait;
use std::sync::Arc;

use tokio::io;
use tokio::sync::{broadcast, watch, Mutex};

#[tokio::main]
async fn main() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let runner = TestRunner::new(2, handler);

    let (tx, mut rx) = broadcast::channel::<()>(32);

    let shutdown: Arc<Mutex<Shutdown>> = Arc::new(Mutex::new(Shutdown {
        shutdown: false,
        notify: rx,
    }));

    let result = runner.run(shutdown).await;

    result.expect("oops something went wrong");
}

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

pub type Handler = fn() -> io::Result<()>;

pub fn run(h: Handler) -> io::Result<()> {
    h()?;
    Ok(())
}

pub struct TestRunner {
    concurrency: i32,
    handler: Handler,
}

impl TestRunner {
    pub fn new(concurrency: i32, handler: Handler) -> Box<TestRunner> {
        Box::new(TestRunner {
            concurrency,
            handler,
        })
    }

    pub async fn run(&self, shutdown: Arc<Mutex<Shutdown>>) -> io::Result<()> {
        // pub async fn run<T: Stopper + Send + Sync + 'static + ?Sized>(
        //     &self,
        //     shutdown: Arc<Mutex<T>>,
        // ) -> io::Result<()> {
        let (tx, rx) = watch::channel(true);

        for i in 0..self.concurrency {
            let mut rx = rx.clone();
            let h = self.handler;
            let x = Arc::clone(&shutdown);

            tokio::spawn(async move {
                let mut shutdown = x.lock().await;

                while !shutdown.is_shutdown() {
                    tokio::select! {
                        _ = rx.changed() => {
                            h().expect("failed processing")
                        }
                        _ = shutdown.recv() => {
                            // The shutdown signal has been received.
                            println!("exiting worker {}", i);
                            return
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
