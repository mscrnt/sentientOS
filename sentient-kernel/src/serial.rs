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
const LINE_STATUS_REGISTER: u16 = COM1_BASE + 5;
#[cfg(feature = "serial-debug")]
const LINE_STATUS_TRANSMIT_EMPTY: u8 = 0x20;

#[cfg(feature = "serial-debug")]
pub struct SerialPort {
    base: u16,
}

#[cfg(feature = "serial-debug")]
impl SerialPort {
    const fn new(base: u16) -> Self {
        SerialPort { base }
    }

    pub fn init(&mut self) {
        unsafe {
            // Already initialized by bootloader, but ensure settings
            outb(self.base + 1, 0x00); // Disable interrupts
            outb(self.base + 3, 0x80); // Enable DLAB
            outb(self.base, 0x03); // Divisor low (38400 baud)
            outb(self.base + 1, 0x00); // Divisor high
            outb(self.base + 3, 0x03); // 8N1
            outb(self.base + 2, 0xC7); // Enable FIFO
            outb(self.base + 4, 0x0B); // Enable RTS/DSR
        }
    }

    fn write_byte(&mut self, byte: u8) {
        unsafe {
            while (inb(self.base + 5) & LINE_STATUS_TRANSMIT_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            outb(self.base, byte);
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
static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1_BASE));

// Public API - always available but no-ops when serial-debug is disabled

pub fn init() {
    #[cfg(feature = "serial-debug")]
    SERIAL1.lock().init();
}

pub fn _print(args: fmt::Arguments) {
    #[cfg(feature = "serial-debug")]
    SERIAL1.lock().write_fmt(args).unwrap();
    
    // When serial is disabled, just drop the output
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
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[allow(dead_code)]
pub fn println(args: fmt::Arguments) {
    _print(args);
    _print(format_args!("\n"));
}

pub fn try_read_char() -> Option<char> {
    #[cfg(feature = "serial-debug")]
    unsafe {
        let line_status = inb(LINE_STATUS_REGISTER);
        if line_status & 0x01 != 0 {
            // Data available
            let byte = inb(DATA_REGISTER);
            Some(byte as char)
        } else {
            None
        }
    }
    
    #[cfg(not(feature = "serial-debug"))]
    None
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