use anyhow::{Context, Result};
use serialport::{SerialPort, SerialPortType};
use std::io::{self, BufRead, BufReader, Write};
use std::time::Duration;

use crate::{shell_state::ShellState, BANNER};

pub fn run_serial_shell() -> Result<()> {
    // Try to find the serial port
    let port_name = find_serial_port()?;
    log::info!("Using serial port: {}", port_name);

    // Open the serial port
    let port = serialport::new(&port_name, 115200)
        .timeout(Duration::from_millis(100))
        .open()
        .context("Failed to open serial port")?;

    run_shell_on_port(port)
}

fn find_serial_port() -> Result<String> {
    // First check environment variable
    if let Ok(port) = std::env::var("SENTIENT_SERIAL_PORT") {
        return Ok(port);
    }

    // Try to auto-detect
    let ports = serialport::available_ports().context("Failed to list serial ports")?;

    for port in ports {
        match port.port_type {
            SerialPortType::UsbPort(_) => {
                // Prefer USB serial ports
                return Ok(port.port_name);
            }
            _ => {}
        }
    }

    // Default to common serial port names
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/ttyS0").exists() {
            return Ok("/dev/ttyS0".to_string());
        }
        if std::path::Path::new("/dev/ttyUSB0").exists() {
            return Ok("/dev/ttyUSB0".to_string());
        }
    }

    #[cfg(target_os = "windows")]
    {
        return Ok("COM1".to_string());
    }

    anyhow::bail!("No suitable serial port found")
}

pub fn run_shell_on_port(mut port: Box<dyn SerialPort>) -> Result<()> {
    // Send banner
    writeln!(port, "{}", BANNER)?;
    writeln!(port, "Type 'help' for available commands.\n")?;

    let mut shell = ShellState::new();
    let mut reader = BufReader::new(port.try_clone()?);

    loop {
        // Send prompt
        write!(port, "sentient> ")?;
        port.flush()?;

        // Read command
        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(0) => {
                // EOF
                break;
            }
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                // Echo the command (since serial might not echo)
                writeln!(port, "{}", input)?;

                // Execute command and capture output
                match execute_with_output(&mut shell, input) {
                    Ok((output, should_exit)) => {
                        // Send output over serial
                        for line in output.lines() {
                            writeln!(port, "{}", line)?;
                        }

                        if should_exit {
                            writeln!(port, "Goodbye!")?;
                            break;
                        }
                    }
                    Err(e) => {
                        writeln!(port, "Error: {}", e)?;
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                // Timeout is ok, just continue
                continue;
            }
            Err(e) => {
                log::error!("Serial read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

// Execute command and capture output as string
fn execute_with_output(shell: &mut ShellState, input: &str) -> Result<(String, bool)> {
    use std::sync::{Arc, Mutex};

    let output = Arc::new(Mutex::new(String::new()));
    let output_clone = Arc::clone(&output);

    // Temporarily redirect stdout to capture output
    let old_stdout = std::io::stdout();

    // Create a custom writer that captures output
    struct CaptureWriter {
        output: Arc<Mutex<String>>,
    }

    impl Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let s = String::from_utf8_lossy(buf);
            self.output.lock().unwrap().push_str(&s);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // For simplicity, we'll execute the command and capture any println! output
    // In a real implementation, we'd properly redirect stdout
    let result = shell.execute_command(input)?;

    // For now, we'll handle output directly in the command handlers
    // This is a simplified version - in production you'd use proper output capture
    Ok((String::new(), result))
}

// Helper function for kernel integration
pub fn create_serial_shell_for_kernel() -> Result<Box<dyn SerialPort>> {
    // When running in kernel/QEMU, use specific settings
    let port = serialport::new("/dev/ttyS0", 115200)
        .timeout(Duration::from_millis(100))
        .open()
        .context("Failed to open kernel serial port")?;

    Ok(port)
}
