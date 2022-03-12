use std::sync::Arc;
use super::shutdown;
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
        producer_events: &ProducerEvents,
    ) -> io::Result<()> {
        let ticker_killed = producer_events.on_producer_killed.clone();
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut signaler = shutdown.get_signaler();
        let mut rate_calculator: RegularRateCalculator = RateCalculator::new(self.rate_per_sec);

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

pub type ProducerKilledCallback = Box<dyn Fn() + Send + Sync>;

pub struct ProducerEvents {
    pub on_producer_killed: Arc<ProducerKilledCallback>,
}

trait RateCalculator {
    fn new(rate_per_sec: f32) -> Self;
    fn next_sec(&mut self) -> i32;
}

struct RegularRateCalculator {
    acc: f32,
    count_per_sec: f32,
}

impl RateCalculator for RegularRateCalculator {
    fn new(rate_per_sec: f32) -> RegularRateCalculator {
        RegularRateCalculator {
            acc: 0.0,
            count_per_sec: rate_per_sec,
        }
    }

    fn next_sec(&mut self) -> i32 {
        let chunk = self.count_per_sec as f32 / 10.0;
        let chunk_round = (chunk * 10_000_000.0).round() / 10_000_000.0;
        self.acc += chunk_round;
        if self.acc >= 1.0 {
            let r = self.acc;
            self.acc = r.rem_euclid(r.floor());
            r.floor() as i32
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! regular_rate_calc_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (count_per_sec, iter, expected) = $value;
                let mut rate_calc = RegularRateCalculator::new(count_per_sec);
                let result: Vec<_> = (0..iter).into_iter().map(|_| rate_calc.next_sec()).collect();
                assert_eq!(expected, result);
            }
        )*
        }
    }

    regular_rate_calc_tests! {
        // more than one per sec
        one_per_sec: (1.0, 10, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
        ten_per_sec: (10.0, 10, vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        fifteen_per_sec: (15.0, 10, vec![1, 2, 1, 2, 1, 2, 1, 2, 1, 2]),
        twentytwo_per_sec: (22.0, 10, vec![2, 2, 2, 2, 3, 2, 2, 2, 2, 3]),
        one_hundred_per_sec: (100.0, 10, vec![10, 10, 10, 10, 10, 10, 10, 10, 10, 10]),

        // less than one per sec
        zero_per_sec: (0.0, 10, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        half_per_sec: (0.5, 20, seq_with_1_at_index(19)),
        third_per_sec: (0.333, 31, seq_with_1_at_index(30)),
    }

    fn seq_with_1_at_index(i: usize) -> Vec<i32> {
        vec![0].into_iter().cycle().take(i).chain(vec![1].into_iter()).collect()
    }
}
