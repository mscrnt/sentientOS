use x86_64::instructions::port::Port;
use spin::Mutex;

static TIMER_TICKS: Mutex<u64> = Mutex::new(0);

const PIT_FREQUENCY: u32 = 1193182;
const DESIRED_FREQUENCY: u32 = 1000; // 1000 Hz = 1ms per tick

pub fn init() {
    // Configure PIT (Programmable Interval Timer)
    let divisor = PIT_FREQUENCY / DESIRED_FREQUENCY;
    
    unsafe {
        // Send command byte
        Port::<u8>::new(0x43).write(0x36);
        
        // Send frequency divisor
        Port::<u8>::new(0x40).write((divisor & 0xFF) as u8);
        Port::<u8>::new(0x40).write((divisor >> 8) as u8);
    }
}

pub fn get_time_ms() -> u64 {
    // For now, use UEFI's time services if available
    // In a real implementation, we'd use RDTSC or HPET
    *TIMER_TICKS.lock()
}

pub fn tick() {
    *TIMER_TICKS.lock() += 1;
}