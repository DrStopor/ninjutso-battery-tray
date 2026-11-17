//! Local stderr diagnostics; no files, network logging or telemetry.
struct Stderr;
static LOGGER: Stderr = Stderr;
impl log::Log for Stderr {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }
    fn flush(&self) {}
}
pub(super) fn init() {
    let level = match std::env::var("NINJUTSO_LOG").as_deref() {
        Ok("debug") => log::LevelFilter::Debug,
        Ok("info") => log::LevelFilter::Info,
        _ => log::LevelFilter::Warn,
    };
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}
