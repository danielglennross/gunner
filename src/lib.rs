use std::error;

pub type Handler = fn() -> Result<(), Box<dyn error::Error>>;

pub fn run(h: Handler) -> Result<(), Box<dyn error::Error>> {
    h()?;
    Ok(())
}
