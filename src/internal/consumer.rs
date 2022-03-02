use crate::internal::{shutdown, RunEvents};
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
            let signaler = shutdown.get_signaler();
            let processor_killed = run_events.on_processor_killed.clone();
            let h = self.handler;

            tokio::spawn(async move {
                // take ownership of signaler
                // so shutdown knows when this goes out of scope when the thread exists
                let _ = signaler;

                loop {
                    let v = rx.recv().await;
                    match v {
                        Ok(v) => {
                            println!("value is: {}", v);
                            h().expect("failed processing")
                        }
                        Err(async_channel::RecvError) => {
                            println!("channel closed");
                            break;
                        }
                    }
                }

                // The shutdown signal has been received.
                println!("exiting consumer {}", i);
                processor_killed(i);
            });
        }

        Ok(())
    }
}
