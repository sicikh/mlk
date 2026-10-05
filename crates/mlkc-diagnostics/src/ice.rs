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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::Ice;

    #[test]
    fn new_keeps_the_message_and_the_place_it_was_given() {
        let ice = Ice::new(
            "a resolved name without a declaration".to_owned(),
            "crates/mlkc-resolve/src/name.rs",
            42,
            7,
        );

        assert_eq!(ice.message(), "a resolved name without a declaration");
        assert_eq!(ice.location(), "crates/mlkc-resolve/src/name.rs:42:7");
        assert!(!ice.backtrace().to_string().trim().is_empty());
    }

    #[test]
    fn display_reports_the_message_the_place_and_that_it_is_a_bug() {
        let ice = Ice::new("an assumption broke".to_owned(), "src/pass.rs", 9, 1);

        let rendered = ice.to_string();

        assert!(rendered.starts_with("an assumption broke\n  at src/pass.rs:9:1\n"));
        assert!(
            rendered.ends_with("  = note: this is a bug of the compiler, and not of your program")
        );
    }

    #[test]
    fn an_ice_is_an_error_a_host_can_carry() {
        let ice = Ice::new("an assumption broke".to_owned(), "src/pass.rs", 9, 1);

        let error: &dyn Error = &ice;

        assert!(error.source().is_none());
    }

    #[test]
    fn of_gives_an_ice_payload_back_at_its_own_place() {
        let caught = Ice::of(Box::new(Ice::new(
            "an assumption broke".to_owned(),
            "src/pass.rs",
            9,
            1,
        )));

        assert_eq!(caught.message(), "an assumption broke");
        assert_eq!(caught.location(), "src/pass.rs:9:1");
    }

    #[test]
    fn of_reads_a_str_payload_the_way_a_panic_writes_it() {
        let caught = Ice::of(Box::new("index out of bounds"));

        assert_eq!(caught.message(), "index out of bounds");
        assert_eq!(caught.location(), "<unknown>:0:0");
    }

    #[test]
    fn of_reads_a_string_payload() {
        let caught = Ice::of(Box::new(String::from("assertion failed")));

        assert_eq!(caught.message(), "assertion failed");
        assert_eq!(caught.location(), "<unknown>:0:0");
    }

    #[test]
    fn of_names_a_payload_it_cannot_read() {
        let caught = Ice::of(Box::new(42_u32));

        assert_eq!(
            caught.message(),
            "<a panic with a payload the compiler cannot read>"
        );
        assert_eq!(caught.location(), "<unknown>:0:0");
    }

    #[test]
    fn the_ice_macro_carries_the_message_and_the_place_it_was_written() {
        let caught = std::panic::catch_unwind(|| crate::ice!("no declaration for {}", "Name"))
            .expect_err("the macro to stop the compiler");
        let caught = caught
            .downcast::<Ice>()
            .expect("the payload to be an internal compiler exception");

        assert_eq!(caught.message(), "no declaration for Name");
        assert!(
            caught.location().starts_with(&format!("{}:", file!())),
            "the exception is raised where the macro was written, got {}",
            caught.location()
        );
    }
}
