//! Prints a description of the given cron expression

use saffron::parse::{CronExpr, English};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args
        .get(1)
        .map(|s| s.as_str().parse::<CronExpr>())
        .transpose()
    {
        Ok(Some(cron)) => println!("{}", cron.describe(English::default())),
        Ok(None) => println!("Usage: cargo run --example describe -- \"[cron expression]\""),
        Err(err) => println!("{}", err),
    }
}
