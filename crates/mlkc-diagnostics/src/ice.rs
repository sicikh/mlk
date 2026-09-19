#[macro_export]
macro_rules! ice {
    ($($arg:tt)*) => {{
        $crate::diag::ice_impl(
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
