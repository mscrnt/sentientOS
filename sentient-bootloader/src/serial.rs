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
    initialized: bool,
}

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

            // Test serial chip (send 0xAE and check if we get it back)
            outb(COM1_BASE, 0xAE);
            if inb(COM1_BASE) != 0xAE {
                return; // Serial not working
            }

            // Serial is ready, enable OUT2
            outb(MODEM_CONTROL, 0x0F);
        }

        self.initialized = true;
    }

    fn write_byte(&mut self, byte: u8) {
        if !self.initialized {
            self.init();
        }

        unsafe {
            // Wait for transmit buffer to be empty
            while (inb(LINE_STATUS) & LINE_STATUS_TRANSMIT_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            outb(DATA_REGISTER, byte);
        }
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// Global serial port protected by a mutex
static SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());

pub fn init_serial() {
    SERIAL_PORT.lock().init();
}

pub fn serial_print(args: fmt::Arguments) {
    SERIAL_PORT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::serial_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", format_args!($($arg)*))
    };
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
