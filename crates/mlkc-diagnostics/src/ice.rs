//! The internal compiler exception: what a pass raises when its input breaks its contract.
//!
//! An ICE is a bug of the compiler and not of the program. It is an ordinary panic
//! ([`ice!`]), so a caller that catches nothing gets the usual hook, message, and unwinding;
//! what makes it special is the payload ([`Ice`]), which says what was assumed, where the
//! assumption was broken, and carries the backtrace of it.
//!
//! The driver catches the panic where it calls a pass and turns it into a report a host shows
//! (the `IceReport` of `mlkc-driver`): one buggy body does not have to end a host that compiled
//! thousands of others. A host that would rather die gets the same report from what it caught.

use std::{any::Any, backtrace::Backtrace, error::Error, fmt};

/// What an internal compiler exception says: the message, where it was raised, and a backtrace.
#[derive(Debug)]
pub struct Ice {
    /// What the pass assumed, and what it met instead.
    message: String,
    /// Where the exception was raised.
    file: &'static str,
    /// The line of [`Self::file`].
    line: u32,
    /// The column of [`Self::file`].
    column: u32,
    /// The stack of the panic, captured where it was raised.
    backtrace: Backtrace,
}

impl Ice {
    /// An exception at `file`, `line`, and `column`.
    pub fn new(message: String, file: &'static str, line: u32, column: u32) -> Self {
        Self {
            message,
            file,
            line,
            column,
            backtrace: Backtrace::force_capture(),
        }
    }

    /// The exception a caught panic is.
    ///
    /// A payload of [`Ice`] is an exception the compiler raised on purpose; any other payload is
    /// a panic it did not, and what it says is read the way the standard library writes it.
    pub fn of(payload: Box<dyn Any + Send>) -> Self {
        let (message, backtrace) = match payload.downcast::<Self>() {
            Ok(ice) => return *ice,
            Err(payload) => (message_of(&*payload), Backtrace::force_capture()),
        };

        Self {
            message,
            // A panic the compiler did not raise has the location the hook prints, and the
            // payload does not carry it: the report says where it can.
            file: "<unknown>",
            line: 0,
            column: 0,
            backtrace,
        }
    }

    /// What the pass assumed, and what it met instead.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the exception was raised: the file, the line, and the column.
    pub fn location(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }

    /// The stack of the panic, captured where it was raised.
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for Ice {
    /// The exception as a person reads it: what was assumed, where, where the compiler was, and
    /// what to do about it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.message)?;
        writeln!(f, "  at {}", self.location())?;
        writeln!(f, "{}", self.backtrace)?;
        write!(
            f,
            "  = note: this is a bug of the compiler, and not of your program"
        )
    }
}

impl Error for Ice {}

/// What a payload that is not an [`Ice`] says.
fn message_of(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_owned();
    }

    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    "<a panic with a payload the compiler cannot read>".to_owned()
}

/// Stops the compiler with an internal compiler exception.
///
/// A pass that meets what its contract says it cannot meet says so with this: the input is one a
/// stage before it should have rejected, so it is a bug of the compiler and not of the program,
/// and what a reader is told is where the bug is.
#[macro_export]
macro_rules! ice {
    ($($arg:tt)*) => {{
        $crate::ice_impl(
            format!($($arg)*),
            file!(),
            line!(),
            column!(),
        )
    }};
}

#[doc(hidden)]
pub fn ice_impl(message: String, file: &'static str, line: u32, column: u32) -> ! {
    std::panic::panic_any(Ice::new(message, file, line, column))
}
