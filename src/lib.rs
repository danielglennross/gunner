// mod shutdown;
//
// use std::io;
// use std::sync::Arc;
// use tokio::sync::{watch, Mutex};

// Runner - spin up workers
// Worker input handler
// Each worker receives from work channel, processes handler
// Each handler writes to metric (observable)

// Scheduler - spawned process
// Calculates rate, sends work signal

// Processor - spawned process
// Runs on interval reporting metrics

// pub type Handler = fn() -> io::Result<()>;
//
// pub fn run(h: Handler) -> io::Result<()> {
//     h()?;
//     Ok(())
// }
//
// pub struct TestRunner {
//     concurrency: i32,
//     handler: Handler,
// }
//
// impl TestRunner {
//     pub fn new(concurrency: i32, handler: Handler) -> Box<TestRunner> {
//         Box::new(TestRunner {
//             concurrency,
//             handler,
//         })
//     }
//
//     pub async fn run(&self, shutdown: Arc<Mutex<shutdown::Shutdown>>) -> io::Result<()> {
//         // pub async fn run<T: Stopper + Send + Sync + 'static + ?Sized>(
//         //     &self,
//         //     shutdown: Arc<Mutex<T>>,
//         // ) -> io::Result<()> {
//         let (tx, rx) = watch::channel(true);
//
//         for i in 0..self.concurrency {
//             let mut rx = rx.clone();
//             let h = self.handler;
//             let x = Arc::clone(&shutdown);
//
//             tokio::spawn(async move {
//                 let mut shutdown = x.lock().await;
//
//                 while !shutdown.is_shutdown() {
//                     tokio::select! {
//                         _ = rx.changed() => {
//                             h().expect("failed processing")
//                         }
//                         _ = shutdown.recv() => {
//                             // The shutdown signal has been received.
//                             println!("exiting worker {}", i);
//                             return
//                         }
//                     }
//                 }
//             });
//         }
//
//         Ok(())
//     }
// }
