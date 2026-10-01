//! The internal compiler exception: what a pass says when its input breaks its contract.

/// Stops the compiler with an internal compiler exception.
///
/// A pass that meets what its contract says it cannot meet says so with this: the input is one
/// a stage before it should have rejected, so it is a bug of the compiler and not of the program,
/// and the process ends with where the bug is.
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
pub fn ice_impl(message: String, file: &str, line: u32, column: u32) -> ! {
    let notes = [
        format!("location: {}:{}:{}", file, line, column),
        "this is a bug in the compiler, not in your program".into(),
    ];

    eprintln!();
    eprintln!("internal compiler exception: {}", message);
    for note in notes {
        eprintln!("  = note: {}", note);
    }

    std::process::abort();
}
