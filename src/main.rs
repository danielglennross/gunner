mod internal;

use internal::{handler, shutdown};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::sync::{broadcast, Mutex};

#[tokio::main]
async fn main() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let runner = handler::TestRunner::new(1, handler);

    let (tx, rx) = broadcast::channel::<()>(1);

    let shutdown: Arc<Mutex<shutdown::Shutdown>> = Arc::new(Mutex::new(shutdown::Shutdown {
        shutdown: false,
        notify: rx,
    }));

    let result = runner.run(shutdown).await;

    result.expect("oops something went wrong");

    tokio::time::sleep(Duration::from_secs(5)).await;

    tx.send(()).expect("failed to send shutdown signal");
}
