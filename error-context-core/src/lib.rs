//!

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
        let file = location.file();
        let line = location.line();
        let column = location.column();
        write!(f, "({file}:{line}:{column})\n{backtrace}")?;
        Ok(())
    }
}
