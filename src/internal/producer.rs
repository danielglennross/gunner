use super::shutdown;
use crate::internal::producer::ConstantRateType::{LessThanOnePerSec, MoreThanOnePerSec};
use crate::internal::RunEvents;
use async_channel::Sender;
use std::time::Duration;
use tokio::io;

pub struct TestTicker {
    rate_per_sec: f32,
}

impl TestTicker {
    pub fn new(rate_per_sec: f32) -> Box<TestTicker> {
        Box::new(TestTicker { rate_per_sec })
    }

    pub async fn run(
        &self,
        tx: Sender<bool>,
        shutdown: &shutdown::Shutdown<'_>,
        run_events: &RunEvents,
    ) -> io::Result<()> {
        let ticker_killed = run_events.on_ticker_killed.clone();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut signaler = shutdown.get_signaler();
        let mut rate_calculator: ConstantRateCalculator = RateCalculator::new(self.rate_per_sec);

        tokio::spawn(async move {
            while !signaler.is_shutdown() {
                tokio::select! {
                    _ = signaler.recv() => {
                        break
                    }
                    _ = interval.tick() => {
                        for _ in 0..rate_calculator.iter() {
                            tx.send(true).await.expect("error sending on channel");
                        }
                    }
                }
            }

            // The shutdown signal has been received.
            println!("exiting worker sender");
            ticker_killed();
        });

        Ok(())
    }
}

trait RateCalculator {
    fn new(rate_per_sec: f32) -> Self;
    fn iter(&mut self) -> i32;
}

enum ConstantRateType {
    MoreThanOnePerSec { count_per_sec: i32 },
    LessThanOnePerSec { count: i32, count_reset_index: i32 },
}

struct ConstantRateCalculator {
    rate_type: ConstantRateType,
}

impl RateCalculator for ConstantRateCalculator {
    fn new(rate_per_sec: f32) -> ConstantRateCalculator {
        ConstantRateCalculator {
            rate_type: if rate_per_sec > 1.0 {
                MoreThanOnePerSec {
                    count_per_sec: rate_per_sec.round() as i32,
                }
            } else {
                LessThanOnePerSec {
                    count: 0,
                    count_reset_index: (1.0 / rate_per_sec).round() as i32,
                }
            },
        }
    }

    fn iter(&mut self) -> i32 {
        match &mut self.rate_type {
            ConstantRateType::MoreThanOnePerSec { count_per_sec } => *count_per_sec,
            ConstantRateType::LessThanOnePerSec {
                count,
                count_reset_index,
            } => {
                *count += 1;
                if *count == *count_reset_index {
                    *count = 0;
                    1
                } else {
                    0
                }
            }
        }
    }
}
