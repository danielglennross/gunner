use super::shutdown;
use crate::internal::producer::ConstantRateType::{LessThanOnePerSec, MoreThanOnePerSec};
use crate::internal::RunEvents;
use async_channel::{Receiver, Sender};
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
        recv: Receiver<bool>,
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
                        recv.close();
                        break
                    }
                    _ = interval.tick() => {
                        for _ in 0..rate_calculator.next_sec() {
                            tx.send(true).await.expect("error sending on channel");
                        }
                    }
                }
            }

            // The shutdown signal has been received.
            println!("exiting producer");
            ticker_killed();
        });

        Ok(())
    }
}

trait RateCalculator {
    fn new(rate_per_sec: f32) -> Self;
    fn next_sec(&mut self) -> i32;
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

    fn next_sec(&mut self) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! constant_rate_calc_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                let mut rate_calc = ConstantRateCalculator::new(input);
                let result: Vec<_> = (0..10).into_iter().map(|_| rate_calc.next_sec()).collect();
                assert_eq!(expected, result);
            }
        )*
        }
    }

    constant_rate_calc_tests! {
        // more than one per sec
        one_per_sec: (1.0, vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        ten_per_sec: (10.0, vec![10, 10, 10, 10, 10, 10, 10, 10, 10, 10]),
        one_hundred_per_sec: (100.0, vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 100]),

        // less than one per sec
        zero_per_sec: (0.0, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        half_per_sec: (0.5, vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 1]),
        third_per_sec: (0.33, vec![0, 0, 1, 0, 0, 1, 0, 0, 1, 0]),
        quater_per_sec: (0.25, vec![0, 0, 0, 1, 0, 0, 0, 1, 0, 0]),
    }
}
