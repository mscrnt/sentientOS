use core::fmt;
#[cfg(feature = "serial-debug")]
use core::fmt::Write;

#[cfg(feature = "serial-debug")]
use spin::Mutex;

#[cfg(feature = "serial-debug")]
const COM1_BASE: u16 = 0x3F8;
#[cfg(feature = "serial-debug")]
const DATA_REGISTER: u16 = COM1_BASE;
#[cfg(feature = "serial-debug")]
const INTERRUPT_ENABLE: u16 = COM1_BASE + 1;
#[cfg(feature = "serial-debug")]
const FIFO_CONTROL: u16 = COM1_BASE + 2;
#[cfg(feature = "serial-debug")]
const LINE_CONTROL: u16 = COM1_BASE + 3;
#[cfg(feature = "serial-debug")]
const MODEM_CONTROL: u16 = COM1_BASE + 4;
#[cfg(feature = "serial-debug")]
const LINE_STATUS: u16 = COM1_BASE + 5;
#[cfg(feature = "serial-debug")]
const DIVISOR_LSB: u16 = COM1_BASE;
#[cfg(feature = "serial-debug")]
const DIVISOR_MSB: u16 = COM1_BASE + 1;
#[cfg(feature = "serial-debug")]
const LINE_STATUS_TRANSMIT_EMPTY: u8 = 0x20;

#[cfg(feature = "serial-debug")]
pub struct SerialPort {
    initialized: bool,
}

#[cfg(feature = "serial-debug")]
impl SerialPort {
    const fn new() -> Self {
        SerialPort { initialized: false }
    }

    fn init(&mut self) {
        if self.initialized {
            return;
        }

        unsafe {
            // Disable interrupts
            outb(INTERRUPT_ENABLE, 0x00);

            // Enable DLAB (Divisor Latch Access Bit)
            outb(LINE_CONTROL, 0x80);

            // Set divisor for 38400 baud (divisor = 3)
            outb(DIVISOR_LSB, 0x03);
            outb(DIVISOR_MSB, 0x00);

            // 8 bits, no parity, 1 stop bit (8N1)
            outb(LINE_CONTROL, 0x03);

            // Enable FIFO, clear them, with 14-byte threshold
            outb(FIFO_CONTROL, 0xC7);

            // Enable RTS/DSR
            outb(MODEM_CONTROL, 0x03);

            // Test serial chip
            outb(MODEM_CONTROL, 0x1E);
            if inb(MODEM_CONTROL) != 0x1E {
                // Serial not found
                return;
            }
            outb(MODEM_CONTROL, 0x0F);

            self.initialized = true;
        }
    }

    fn write_byte(&mut self, byte: u8) {
        if !self.initialized {
            return;
        }

        unsafe {
            while (inb(LINE_STATUS) & LINE_STATUS_TRANSMIT_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            outb(DATA_REGISTER, byte);
        }
    }
}

#[cfg(feature = "serial-debug")]
impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

#[cfg(feature = "serial-debug")]
static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new());

// Public API - always available but no-ops when serial-debug is disabled

pub fn init_serial() {
    #[cfg(feature = "serial-debug")]
    {
        SERIAL1.lock().init();
        log_serial("📡 Serial initialized at 38400 8N1");
    }
}

pub fn log_serial(msg: &str) {
    #[cfg(feature = "serial-debug")]
    {
        serial_print!("{}", msg);
        serial_print!("\r\n");
    }
    
    #[cfg(not(feature = "serial-debug"))]
    let _ = msg;
}

pub fn _print(args: fmt::Arguments) {
    #[cfg(feature = "serial-debug")]
    SERIAL1.lock().write_fmt(args).unwrap();
    
    #[cfg(not(feature = "serial-debug"))]
    let _ = args;
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {{
        $crate::serial::_print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => {{
        $crate::serial_print!($($arg)*);
        $crate::serial_print!("\r\n");
    }};
}

#[cfg(feature = "serial-debug")]
#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

#[cfg(feature = "serial-debug")]
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}