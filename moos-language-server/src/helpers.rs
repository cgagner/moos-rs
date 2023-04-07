use log::{Level, LevelFilter, Metadata, Record};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

#[cfg(not(test))]
pub(crate) fn _log(s: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        console_log(&("[moos-language-server] ".to_owned() + s))
    }
}

#[cfg(test)]
pub(crate) fn _log(_: &str) {}

pub(crate) struct LspLogger;

impl log::Log for LspLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            _log(&format!("{} - {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

static LOGGER: LspLogger = LspLogger;

impl LspLogger {
    pub(crate) fn init(level: LevelFilter) {
        let _result = log::set_logger(&LOGGER).map(|()| log::set_max_level(level));
    }
}
