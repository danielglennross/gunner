use gunner::internal::{
    consumer, producer, shutdown, ProcessorKilledCallback, RunEvents, TickerKilledCallback,
};
use std::sync::{Arc, Mutex};

use gunner::internal::shutdown::CountDownInterrupter;
use tokio::io;

struct TestRunEvents {
    ticker_kill_count: Arc<Mutex<u8>>,
    processor_kill_count: Arc<Mutex<u8>>,
}

impl TestRunEvents {
    pub fn new() -> TestRunEvents {
        TestRunEvents {
            ticker_kill_count: Arc::new(Mutex::new(0)),
            processor_kill_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn on_ticker_killed(&self) -> Arc<TickerKilledCallback> {
        let c = self.ticker_kill_count.clone();
        Arc::new(Box::new(move || {
            let mut count = c.lock().unwrap();
            *count += 1;
        }))
    }

    pub fn on_processor_killed(&self) -> Arc<ProcessorKilledCallback> {
        let c = self.processor_kill_count.clone();
        Arc::new(Box::new(move |_| {
            let mut count = c.lock().unwrap();
            *count += 1;
        }))
    }

    pub fn get_ticker_killed_count(&self) -> u8 {
        *self.ticker_kill_count.lock().unwrap()
    }

    pub fn get_processor_killed_count(&self) -> u8 {
        *self.processor_kill_count.lock().unwrap()
    }
}

#[tokio::test]
async fn graceful_shutdown() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let rate_per_sec = 1.0;

    let processor_count: u8 = 2;

    let ticker = producer::TestTicker::new(rate_per_sec);

    let runner = consumer::TestRunner::new(processor_count, handler);

    let count_down_interrupter = Box::new(CountDownInterrupter::new(1_000));

    let shutdown = shutdown::Shutdown::new(vec![count_down_interrupter]);

    let test_run_events = TestRunEvents::new();

    let run_events = RunEvents {
        on_ticker_killed: test_run_events.on_ticker_killed(),
        on_processor_killed: test_run_events.on_processor_killed(),
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

    let actual_ticker_count = test_run_events.get_ticker_killed_count();
    let actual_processor_count = test_run_events.get_processor_killed_count();

    assert_eq!(1, actual_ticker_count, "ticker count");
    assert_eq!(processor_count, actual_processor_count, "processor count");
}
