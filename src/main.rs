mod internal;

use internal::{handler, shutdown};
use tokio::io;

#[tokio::main]
async fn main() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let runner = handler::TestRunner::new(1, handler);

    let shutdown = shutdown::Shutdown::new();

    let result = runner.run(&shutdown).await;

    result.expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;
}
