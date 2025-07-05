use crate::serial_println;
use uefi::proto::console::text::{Color, Output};
use uefi::CStr16;

#[derive(Debug, Clone, Copy)]
pub enum Phase {
    Boot,
    Ai,
    Load,
    Exec,
    Error,
}

impl Phase {
    fn color(&self) -> Color {
        match self {
            Phase::Boot => Color::Blue,
            Phase::Ai => Color::Magenta,
            Phase::Load => Color::Yellow,
            Phase::Exec => Color::Green,
            Phase::Error => Color::Red,
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Phase::Boot => "🔵",
            Phase::Ai => "🟣",
            Phase::Load => "🟡",
            Phase::Exec => "🟢",
            Phase::Error => "🔴",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Phase::Boot => "BOOT",
            Phase::Ai => "AI",
            Phase::Load => "LOAD",
            Phase::Exec => "EXEC",
            Phase::Error => "ERROR",
        }
    }
}

pub fn print_phase(stdout: &mut Output, phase: Phase, message: &str) {
    // Display on VGA
    stdout.set_color(phase.color(), Color::Black).unwrap();

    let phase_str = alloc::format!("[{}] ", phase.name());
    let phase_u16 = crate::str_to_cstr16(&phase_str);
    let phase_cstr = CStr16::from_u16_with_nul(&phase_u16).unwrap();
    stdout.output_string(phase_cstr).unwrap();

    stdout.reset(false).unwrap();

    let msg_str = alloc::format!("{}\r\n", message);
    let msg_u16 = crate::str_to_cstr16(&msg_str);
    let msg_cstr = CStr16::from_u16_with_nul(&msg_u16).unwrap();
    stdout.output_string(msg_cstr).unwrap();

    // Also send to serial
    serial_println!("{} [{}] {}", phase.symbol(), phase.name(), message);
}
