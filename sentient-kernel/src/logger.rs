use log::{LevelFilter, Log, Metadata, Record};

struct KernelLogger;

impl Log for KernelLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_emoji = match record.level() {
                log::Level::Error => "🔴",
                log::Level::Warn => "🟡",
                log::Level::Info => "🔵",
                log::Level::Debug => "🟢",
                log::Level::Trace => "⚪",
            };
            
            serial_println!("{} [{}] {}", 
                level_emoji,
                record.level(), 
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: KernelLogger = KernelLogger;

pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}