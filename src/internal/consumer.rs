use crate::internal::shutdown;
use async_channel::Receiver;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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
        consumer_events: &ConsumerEvents,
    ) -> io::Result<()> {
        let total_workers: usize = self.concurrency as usize;
        let busy_worker = Arc::new(AtomicUsize::new(0));

        let (g_tx, g_rx) = async_channel::unbounded::<i64>();

        // governor
        let rx = rx.clone();
        let signaler = shutdown.get_signaler();
        let busy_workers_gov = Arc::clone(&busy_worker);
        tokio::spawn(async move {
            // take ownership of signaler
            // so shutdown knows when this goes out of scope when the thread exists
            let _ = signaler;

            let iteration: i64 = 0;
            loop {
                let v = rx.recv().await;

                match v {
                    Ok(v) => {
                        if busy_workers_gov.load(Ordering::SeqCst) >= total_workers {
                            continue;
                        }
                        g_tx.send(iteration)
                            .await
                            .expect("error sending on channel");
                    }
                    Err(async_channel::RecvError) => {
                        g_tx.close();
                        println!("governor channel closed");
                        break;
                    }
                }
            }
        });

        // workers
        for i in 0..total_workers {
            let grx = g_rx.clone();
            let signaler = shutdown.get_signaler();
            let busy_workers = Arc::clone(&busy_worker);
            let processor_killed = consumer_events.on_worker_killed.clone();
            let work_handled = consumer_events.on_work_handled.clone();
            let h = self.handler;

            tokio::spawn(async move {
                // take ownership of signaler
                // so shutdown knows when this goes out of scope when the thread exists
                let _ = signaler;

                loop {
                    let v = grx.recv().await;

                    match v {
                        Ok(v) => {
                            busy_workers.fetch_add(1, Ordering::SeqCst);
                            println!("value is: {}", v);
                            match h() {
                                Ok(()) => work_handled(i, io::Result::Ok(())),
                                Err(e) => work_handled(i, io::Result::Err(e)),
                            }
                            busy_workers.fetch_sub(1, Ordering::SeqCst);
                        }
                        Err(async_channel::RecvError) => {
                            println!("consumer channel {} closed", i);
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

pub type WorkerKilledCallback = Box<dyn Fn(usize) + Send + Sync>;
pub type WorkHandledCallback = Box<dyn Fn(usize, io::Result<()>) + Send + Sync>;

pub struct ConsumerEvents {
    pub on_worker_killed: Arc<WorkerKilledCallback>,
    pub on_work_handled: Arc<WorkHandledCallback>,
}
