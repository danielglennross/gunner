use gunner::internal::{consumer, producer, shutdown};
use std::sync::{Arc, Mutex};

use gunner::internal::consumer::{ConsumerEvents, WorkHandledCallback, WorkerKilledCallback};
use gunner::internal::producer::{ProducerEvents, ProducerKilledCallback};
use gunner::internal::shutdown::CountDownInterrupter;
use tokio::io;

struct TestProducerEvents {
    producer_kill_count: Arc<Mutex<u8>>,
}

struct TestConsumerEvents {
    worker_kill_count: Arc<Mutex<u8>>,
    worker_handle_count: Arc<Mutex<Vec<u8>>>,
}

impl TestProducerEvents {
    pub fn new() -> TestProducerEvents {
        TestProducerEvents {
            producer_kill_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn on_producer_killed(&self) -> Arc<ProducerKilledCallback> {
        let c = self.producer_kill_count.clone();
        Arc::new(Box::new(move || {
            let mut count = c.lock().unwrap();
            *count += 1;
        }))
    }

    pub fn get_producer_killed_count(&self) -> u8 {
        *self.producer_kill_count.lock().unwrap()
    }
}

impl TestConsumerEvents {
    pub fn new() -> TestConsumerEvents {
        TestConsumerEvents {
            worker_kill_count: Arc::new(Mutex::new(0)),
            worker_handle_count: Arc::new(Mutex::new(vec![0; 2])),
        }
    }

    pub fn on_worker_killed(&self) -> Arc<WorkerKilledCallback> {
        let c = self.worker_kill_count.clone();
        Arc::new(Box::new(move |_| {
            let mut count = c.lock().unwrap();
            *count += 1;
        }))
    }

    pub fn get_worker_killed_count(&self) -> u8 {
        *self.worker_kill_count.lock().unwrap()
    }

    pub fn on_worker_handled(&self) -> Arc<WorkHandledCallback> {
        let c = self.worker_handle_count.clone();
        Arc::new(Box::new(move |worker_index, _| {
            let mut wc = c.lock().unwrap();
            wc[worker_index] += 1;
        }))
    }

    pub fn get_worker_handled_counts(&self) -> Vec<u8> {
        self.worker_handle_count.lock().unwrap().clone()
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

    let test_producer_events = TestProducerEvents::new();
    let test_consumer_events = TestConsumerEvents::new();

    let producer_events = ProducerEvents {
        on_producer_killed: test_producer_events.on_producer_killed(),
    };

    let consumer_events = ConsumerEvents {
        on_worker_killed: test_consumer_events.on_worker_killed(),
        on_work_handled: test_consumer_events.on_worker_handled(),
    };

    let (tx, rx) = async_channel::unbounded::<bool>();

    ticker
        .run(tx, rx.clone(), &shutdown, &producer_events)
        .await
        .expect("oops something went wrong, ticker.run");

    runner
        .run(rx, &shutdown, &consumer_events)
        .await
        .expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;

    let actual_producer_count = test_producer_events.get_producer_killed_count();
    let actual_worker_count = test_consumer_events.get_worker_killed_count();

    assert_eq!(1, actual_producer_count, "producer count");
    assert_eq!(processor_count, actual_worker_count, "worker count");
}

#[tokio::test]
async fn worker_handled_count() {
    let handler = || -> io::Result<()> {
        println!("Hello, world!");
        Ok(())
    };

    let rate_per_sec = 10.0;

    let processor_count: u8 = 2;

    let ticker = producer::TestTicker::new(rate_per_sec);

    let runner = consumer::TestRunner::new(processor_count, handler);

    let count_down_interrupter = Box::new(CountDownInterrupter::new(1_000));

    let shutdown = shutdown::Shutdown::new(vec![count_down_interrupter]);

    let test_producer_events = TestProducerEvents::new();
    let test_consumer_events = TestConsumerEvents::new();

    let producer_events = ProducerEvents {
        on_producer_killed: test_producer_events.on_producer_killed(),
    };

    let consumer_events = ConsumerEvents {
        on_worker_killed: test_consumer_events.on_worker_killed(),
        on_work_handled: test_consumer_events.on_worker_handled(),
    };

    let (tx, rx) = async_channel::unbounded::<bool>();

    ticker
        .run(tx, rx.clone(), &shutdown, &producer_events)
        .await
        .expect("oops something went wrong, ticker.run");

    runner
        .run(rx, &shutdown, &consumer_events)
        .await
        .expect("oops something went wrong, runner.run");

    shutdown.register_shutdown().await;

    let actual_worker_handled_count = test_consumer_events.get_worker_handled_counts();
    let sum = actual_worker_handled_count.iter().map(|s| *s as i32).sum();

    assert!((10i32..12i32).contains(&sum));
}
