//! Prints an list of times from a cron iterator until the DateTime container is maxed out

use chrono::Utc;
use saffron::Cron;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str().parse::<Cron>()).transpose() {
        Ok(Some(cron)) => {
            if !cron.any() {
                println!("Cron will never match any given time!");
                return;
            }

            let futures = cron.clone().iter_from(Utc::now());
            for time in futures {
                if !cron.contains(time) {
                    println!("Failed check! Cron does not contain {}.", time);
                    break;
                }
                println!("{}", time.format("%F %R"));
            }
        }
        Ok(None) => println!("Usage: cargo run --example future-times -- \"[cron expression]\""),
        Err(err) => println!("{}", err),
    }
}
