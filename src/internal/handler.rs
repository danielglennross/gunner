use super::shutdown;

use std::time::Duration;
use tokio::io;

pub type Handler = fn() -> io::Result<()>;

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

    pub async fn run(&self, shutdown: &shutdown::Shutdown) -> io::Result<()> {
        let (tx, rx) = async_channel::unbounded();

        let mut signaler1 = shutdown.get_signaler();
        tokio::spawn(async move {
            while !signaler1.is_shutdown() {
                tokio::select! {
                    _ = signaler1.recv() => {
                        // The shutdown signal has been received.
                        println!("exiting worker sender");
                        return
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        tx.send(true).await.expect("error sending on channel");
                    }
                }
            }
        });

        for i in 0..self.concurrency {
            let rx = rx.clone();
            let h = self.handler;
            let mut signaler2 = shutdown.get_signaler();

            tokio::spawn(async move {
                while !signaler2.is_shutdown() {
                    tokio::select! {
                        _ = signaler2.recv() => {
                            // The shutdown signal has been received.
                            println!("exiting worker {}", i);
                            return
                        }
                        v = rx.recv() => {
                            match v {
                                Ok(v) => {
                                    println!("value is: {}", v);
                                    h().expect("failed processing")
                                },
                                Err(async_channel::RecvError) => {
                                   println!("channel closed")
                                },
                            }
                        }
                        else => continue
                    }
                }
            });
        }

        Ok(())
    }
}
