use core::fmt::{self, Write};
use spin::Mutex;

const COM1_BASE: u16 = 0x3F8;
const DATA_REGISTER: u16 = COM1_BASE + 0;
const INTERRUPT_ENABLE: u16 = COM1_BASE + 1;
const FIFO_CONTROL: u16 = COM1_BASE + 2;
const LINE_CONTROL: u16 = COM1_BASE + 3;
const MODEM_CONTROL: u16 = COM1_BASE + 4;
const LINE_STATUS: u16 = COM1_BASE + 5;
const DIVISOR_LSB: u16 = COM1_BASE + 0;
const DIVISOR_MSB: u16 = COM1_BASE + 1;

const LINE_STATUS_TRANSMIT_EMPTY: u8 = 0x20;

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    const fn new(base: u16) -> Self {
        SerialPort { base }
    }

    pub fn init(&mut self) {
        unsafe {
            // Already initialized by bootloader, but ensure settings
            outb(self.base + 1, 0x00); // Disable interrupts
            outb(self.base + 3, 0x80); // Enable DLAB
            outb(self.base + 0, 0x03); // Divisor low (38400 baud)
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

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1_BASE));

pub fn init() {
    SERIAL1.lock().init();
}

pub fn _print(args: fmt::Arguments) {
    SERIAL1.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

pub fn println(args: fmt::Arguments) {
    _print(args);
    _print(format_args!("\n"));
}

#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

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
