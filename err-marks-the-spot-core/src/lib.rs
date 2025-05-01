//!

use ansi_term::Color;

#[derive(Debug)]
pub struct ErrorCtx {
    location: &'static std::panic::Location<'static>,
    backtrace: std::backtrace::Backtrace,
}

impl ErrorCtx {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

#[rustfmt::skip]
impl std::fmt::Display for ErrorCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { location, backtrace } = self;
        let error = Color::Red.paint("ERROR");
        let file = {
            let file = location.file();
            Color::Blue.paint(format!("{file}"))
        };
        let line = {
            let line = location.line();
            Color::Green.paint(format!("{line}"))
        };
        let column = {
            let column = location.column();
            Color::Yellow.paint(format!("{column}"))
        };
        writeln!(f, "{error} detected @ {file}:{line}:{column}:")?;
        writeln!(f, "{backtrace}")?;
        Ok(())
    }
}
