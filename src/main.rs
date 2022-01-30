use gunner::run;
use std::error;

fn main() {
    let test = || -> Result<(), Box<dyn error::Error>> {
        println!("Hello, world!");
        Ok(())
    };

    let result = run(test);

    result.expect("oops something went wrong");
}
