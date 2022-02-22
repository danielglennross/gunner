use std::sync::Arc;
use super::shutdown;

use std::time::Duration;
use tokio::io;

pub type Handler = fn() -> io::Result<()>;
pub type RunCallback = Box<dyn Fn() + Send + Sync>;

pub struct TestRunner {
    concurrency: u8,
    handler: Handler,
}

pub struct RunEvents {
    pub on_ticker_killed: Arc<RunCallback>,
    pub on_processor_killed: Arc<RunCallback>
}

impl TestRunner {
    pub fn new(concurrency: u8, handler: Handler) -> Box<TestRunner> {
        Box::new(TestRunner {
            concurrency,
            handler,
        })
    }

    pub async fn run(&self, shutdown: &shutdown::Shutdown<'_>, run_events: RunEvents) -> io::Result<()> {
        let (tx, rx) = async_channel::unbounded();

        let ticker_killed = run_events.on_ticker_killed.clone();

        let mut signaler1 = shutdown.get_signaler();
        tokio::spawn(async move {
            while !signaler1.is_shutdown() {
                tokio::select! {
                    _ = signaler1.recv() => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        tx.send(true).await.expect("error sending on channel");
                    }
                }
            }

            // The shutdown signal has been received.
            println!("exiting worker sender");
            ticker_killed();
        });

        for i in 0..self.concurrency {
            let rx = rx.clone();
            let h = self.handler;
            let mut signaler2 = shutdown.get_signaler();
            let processor_killed = run_events.on_processor_killed.clone();

            tokio::spawn(async move {
                while !signaler2.is_shutdown() {
                    tokio::select! {
                        _ = signaler2.recv() => {
                            break
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

                // The shutdown signal has been received.
                println!("exiting worker {}", i);
                processor_killed();
            });
        }

        Ok(())
    }
}
