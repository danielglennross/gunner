mod internal;

use crate::internal::{producer, RunEvents};
use internal::{consumer, shutdown};
use std::sync::Arc;
use tokio::io;

#[tokio::main]
async fn main() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let rate_per_sec = 1.0;

    let ticker = producer::TestTicker::new(rate_per_sec);

    let runner = consumer::TestRunner::new(1, handler);

    let ctrl_interrupter = Box::new(shutdown::CtrlInterrupter::new());

    let count_down_interrupter = Box::new(shutdown::CountDownInterrupter::new(5_000));

    let shutdown = shutdown::Shutdown::new(vec![ctrl_interrupter, count_down_interrupter]);

    let run_events = RunEvents {
        on_ticker_killed: Arc::new(Box::new(|| {})),
        on_processor_killed: Arc::new(Box::new(|_| {})),
    };

    let (tx, rx) = async_channel::unbounded::<bool>();

    ticker
        .run(tx, rx.clone(), &shutdown, &run_events)
        .await
        .expect("oops something went wrong, ticker.run");

    runner
        .run(rx, &shutdown, &run_events)
        .await
        .expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;
}
