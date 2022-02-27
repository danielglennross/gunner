use super::shutdown;

use crate::internal::RunEvents;
use async_channel::Receiver;
use tokio::io;

pub type Handler = fn() -> io::Result<()>;

pub struct TestRunner {
    concurrency: u8,
    handler: Handler,
}

impl TestRunner {
    pub fn new(concurrency: u8, handler: Handler) -> Box<TestRunner> {
        Box::new(TestRunner {
            concurrency,
            handler,
        })
    }

    pub async fn run(
        &self,
        rx: Receiver<bool>,
        shutdown: &shutdown::Shutdown<'_>,
        run_events: &RunEvents,
    ) -> io::Result<()> {
        for i in 0..self.concurrency {
            let rx = rx.clone();
            let h = self.handler;
            let mut signaler = shutdown.get_signaler();
            let processor_killed = run_events.on_processor_killed.clone();

            tokio::spawn(async move {
                while !signaler.is_shutdown() {
                    tokio::select! {
                        _ = signaler.recv() => {
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
                processor_killed(i);
            });
        }

        Ok(())
    }
}
