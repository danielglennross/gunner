mod internal;

use std::sync::Arc;
use internal::{handler, shutdown};
use tokio::io;

#[tokio::main]
async fn main() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let runner = handler::TestRunner::new(1, handler);

    let ctrl_interrupter = Box::new(shutdown::CtrlInterrupter::new());

    let shutdown = shutdown::Shutdown::new(ctrl_interrupter);

    let run_events = handler::RunEvents{
        on_ticker_killed: Arc::new(Box::new(|| {})),
        on_processor_killed: Arc::new(Box::new(|| {}))
    };

    let result = runner.run(&shutdown, run_events).await;

    result.expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;
}
