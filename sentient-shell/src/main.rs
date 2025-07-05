
use anyhow::Result;
use std::io::{self, Write};

use sentient_shell::{BANNER, SHELL_VERSION, ShellState};

fn main() -> Result<()> {
    env_logger::init();
    
    // Check if we're running in serial mode (for kernel/QEMU)
    let serial_mode = std::env::var("SENTIENT_SERIAL").is_ok();
    
    if serial_mode {
        log::info!("Running in serial mode");
        #[cfg(feature = "serial")]
        return sentient_shell::serial::run_serial_shell();
        
        #[cfg(not(feature = "serial"))]
        {
            log::warn!("Serial mode requested but serial feature not enabled");
            log::info!("Falling back to terminal mode");
        }
    }
    
    log::info!("Running in terminal mode");
    run_terminal_shell()
}

fn run_terminal_shell() -> Result<()> {
    println!("{}", BANNER);
    println!("Type 'help' for available commands.\n");
    
    let mut shell = ShellState::new();
    
    loop {
        print!("sentient> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        
        match shell.execute_command(input) {
            Ok(should_exit) => {
                if should_exit {
                    println!("Goodbye!");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
    
    Ok(())
}

