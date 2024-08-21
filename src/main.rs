use std::process;
use wavrec::run;

fn main() {
    if let Err(err) = run() {
        eprintln!("Application failed with error: {:#?}", err);
        process::exit(1);
    }
}
