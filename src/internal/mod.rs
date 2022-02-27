use std::sync::Arc;

pub mod consumer;
pub mod producer;
pub mod shutdown;

// TODO break these up into producer / consumer modules

pub type TickerKilledCallback = Box<dyn Fn() + Send + Sync>;
pub type ProcessorKilledCallback = Box<dyn Fn(u8) + Send + Sync>;

pub struct RunEvents {
    pub on_ticker_killed: Arc<TickerKilledCallback>,
    pub on_processor_killed: Arc<ProcessorKilledCallback>,
}
