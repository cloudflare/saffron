//! Prints an list of times from a cron iterator until the DateTime container is maxed out

use chrono::Utc;
use saffron::Cron;

const DBG_DEFAULT: &str = "0 0 * * FRI";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.is_empty() {
        println!("Usage: cargo run --example future-times -- \"[cron expression]\"");
        return;
    }

    let cron_str = args.get(1).map(|s| s.as_str()).unwrap_or(DBG_DEFAULT);
    let parsed = cron_str.parse::<Cron>();
    match parsed {
        Ok(cron) => {
            if !cron.any() {
                println!("Cron '{}' will never match any given time!", cron_str);
                return;
            }

            let futures = cron.clone().iter_from(Utc::now());
            for time in futures {
                let result = cron.contains(time);
                if !result {
                    println!("Failed check! {} does not contain {}.", cron_str, time);
                    break;
                }
                println!("{}", time);
            }
        }
        Err(err) => println!("{}", err),
    }
}
