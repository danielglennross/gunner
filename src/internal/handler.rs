use super::shutdown;

use std::sync::Arc;
use std::time::Duration;
use tokio::{io};
use tokio::sync::{watch, Mutex};

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

    pub async fn run(&self, shutdown: Arc<Mutex<shutdown::Shutdown>>) -> io::Result<()> {
        // pub async fn run<T: Stopper + Send + Sync + 'static + ?Sized>(
        //     &self,
        //     shutdown: Arc<Mutex<T>>,
        // ) -> io::Result<()> {
        let (tx, rx) = watch::channel(0);

        tokio::spawn(async move {
            let mut i: i32 = 0;
            //loop {
                tx.send(i).expect("error sending on channel");
            //   i=i+1;
            //}
        });

        for i in 0..self.concurrency {
            let mut rx = rx.clone();
            let h = self.handler;
            let x = Arc::clone(&shutdown);

            tokio::spawn(async move {
                let mut shutdown = x.lock().await;

                while !shutdown.is_shutdown() {
                    tokio::select! {
                        _ = rx.changed() => {
                            let v = rx.borrow_and_update();
                            println!("value is: {}", v.clone());
                            h().expect("failed processing")
                        }
                        _ = shutdown.recv() => {
                            // The shutdown signal has been received.
                            println!("exiting worker {}", i);
                            return
                        }
                        else => continue
                    }
                }
            });
        }

        Ok(())
    }
}
