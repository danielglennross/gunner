use std::sync::{Arc, Mutex};
use gunner::internal::{handler, shutdown};
use std::time::Duration;

use async_trait::async_trait;
use gunner::internal::shutdown::Interrupter;
use tokio::io;

struct CountDownInterrupter {
    ms: u64,
}

impl CountDownInterrupter {
    pub fn new(ms: u64) -> CountDownInterrupter {
        CountDownInterrupter { ms }
    }
}

#[async_trait]
impl Interrupter for CountDownInterrupter {
    async fn wait(&self) -> io::Result<()> {
        Ok(tokio::time::sleep(Duration::from_millis(self.ms)).await)
    }
}

struct TestRunEvents {
    ticker_kill_count: Arc<Mutex<u8>>,
    processor_kill_count: Arc<Mutex<u8>>,
}

impl TestRunEvents {
    pub fn new() -> TestRunEvents {
        TestRunEvents{
            ticker_kill_count: Arc::new(Mutex::new(0)),
            processor_kill_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn on_ticker_killed(&self) -> Arc<handler::RunCallback> {
        let c = self.ticker_kill_count.clone();
        Arc::new(Box::new(move || {
            let mut count = c.lock().unwrap();
            *count += 1;
        }))
    }

    pub fn on_processor_killed(&self) -> Arc<handler::RunCallback> {
        let c = self.processor_kill_count.clone();
        Arc::new(Box::new(move || {
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

    let processor_count: u8 = 1;

    let runner = handler::TestRunner::new(processor_count, handler);

    let count_down_interrupter = Box::new(CountDownInterrupter::new(5_000));

    let shutdown = shutdown::Shutdown::new(count_down_interrupter);

    let test_run_events = TestRunEvents::new();

    let run_events = handler::RunEvents{
        on_ticker_killed: test_run_events.on_ticker_killed(),
        on_processor_killed: test_run_events.on_processor_killed()
    };

    let result = runner.run(&shutdown, run_events).await;

    result.expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;

    let actual_ticker_count = test_run_events.get_ticker_killed_count();
    let actual_processor_count = test_run_events.get_processor_killed_count();

    assert_eq!(1, actual_ticker_count, "ticker count");
    assert_eq!(processor_count, actual_processor_count, "processor count");
}
