use super::shutdown;
use std::borrow::Borrow;
use std::cell::Ref;

use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{broadcast, Mutex};

pub type Handler = fn() -> io::Result<()>;

// pub fn run(h: Handler) -> io::Result<()> {
//     h()?;
//     Ok(())
// }

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

    pub async fn run(&self) -> io::Result<()> {
        // pub async fn run<T: Stopper + Send + Sync + 'static + ?Sized>(
        //     &self,
        //     shutdown: Arc<Mutex<T>>,
        // ) -> io::Result<()> {

        let shutdown = shutdown::Shutdown::new();
        let (tx, rx) = async_channel::unbounded();

        //let shutdown = Arc::new(shutdown);
        //let mut s = Arc::clone(&shutdown);

        let mut shutdown_rx1 = shutdown.get_receiver();

        tokio::spawn(async move {
            loop {
                //while !shutdown.is_shutdown().await {
                //let mut s = x.write().await;
                tokio::select! {
                _ = shutdown_rx1.recv() => {
                    // The shutdown signal has been received.
                    println!("exiting worker sender");
                    return
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    tx.send(true).await.expect("error sending on channel");
                }
                //else => {
                //tx.send(true).await.expect("error sending on channel");
                //tokio::time::sleep(Duration::from_secs(10)).await;
                //}
                }
            }
        });

        for i in 0..self.concurrency {
            let rx = rx.clone();
            let h = self.handler;
            let mut shutdown_rx2 = shutdown.get_receiver();
            //let mut shutdown_rx2 = shutdown_tx.subscribe();
            //let mut shutdown = shutdown.clone();
            //let shutdown = Arc::clone(&shutdown);

            tokio::spawn(async move {
                //let mut x = shutdown;
                loop {
                    //while !shutdown.is_shutdown().await {
                    //loop {
                    tokio::select! {
                        _ = shutdown_rx2.recv() => {
                            // The shutdown signal has been received.
                            println!("exiting worker {}", i);
                            return
                        }
                        v = rx.recv() => {
                            match v {
                                Ok(v) => {
                                    println!("value is: {}", v);
                                },
                                Err(e) => {
                                    println!("error is: {}", e);
                                },
                            }
                            h().expect("failed processing")
                        }
                        else => continue
                    }
                }
            });
        }

        tokio::spawn(async move {
            let ss = Arc::new(shutdown);
            let tt = ss.register();

            loop {
                if tt.is_shutdown().await {
                    println!("we are shutdown");
                    return;
                }

                tokio::time::sleep(Duration::from_secs(5)).await;

                tt.shutdown().expect("failed to send shutdown signal");
            }
        });

        tokio::time::sleep(Duration::from_secs(5)).await;

        //shutdown.shutdown().expect("failed to send shutdown signal");

        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(())
    }
}
