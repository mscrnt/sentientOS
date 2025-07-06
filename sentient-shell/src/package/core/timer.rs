use anyhow::Result;
use std::time::{Duration, Instant};
use std::thread;
use super::{CorePackage, PackageCategory};

pub struct Timer;

impl CorePackage for Timer {
    fn name(&self) -> &'static str { "timer" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Simple timer and stopwatch utility" }
    fn category(&self) -> PackageCategory { PackageCategory::Utils }
}

pub fn run(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }
    
    match args[0] {
        "start" => {
            if args.len() < 2 {
                return Ok("Usage: timer start <seconds>\nExample: timer start 60".to_string());
            }
            let seconds: u64 = args[1].parse()
                .map_err(|_| anyhow::anyhow!("Invalid number of seconds"))?;
            start_timer(seconds)
        },
        "stopwatch" => run_stopwatch(),
        _ => Ok(help()),
    }
}

fn help() -> String {
    "Timer - Simple timer and stopwatch utility\n\
     Commands:\n\
     timer start <seconds>  - Start a countdown timer\n\
     timer stopwatch       - Start a stopwatch (press Enter to stop)".to_string()
}

fn start_timer(seconds: u64) -> Result<String> {
    println!("Timer started for {} seconds...", seconds);
    
    let duration = Duration::from_secs(seconds);
    let start = Instant::now();
    
    // Simple countdown display
    while start.elapsed() < duration {
        let remaining = duration - start.elapsed();
        print!("\rTime remaining: {:02}:{:02}  ", 
               remaining.as_secs() / 60, 
               remaining.as_secs() % 60);
        use std::io::{self, Write};
        io::stdout().flush()?;
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("\n⏰ Timer finished!");
    Ok("Timer completed".to_string())
}

fn run_stopwatch() -> Result<String> {
    println!("Stopwatch started. Press Enter to stop...");
    
    let start = Instant::now();
    
    // Wait for user input
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    let elapsed = start.elapsed();
    let total_secs = elapsed.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let millis = elapsed.subsec_millis();
    
    Ok(format!("Stopwatch stopped at: {:02}:{:02}.{:03}", minutes, seconds, millis))
}