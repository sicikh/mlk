use mlkc_text_size::TextRange;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DiagnosticCategory {
    Parse,
    Resolution,
    Typecheck,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub category: DiagnosticCategory,
    pub range: Option<TextRange>,
    pub message: Message,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
}

// Simple function to kickstart compiler, remove and rework later
pub fn diag(msg: impl Into<String>, category: DiagnosticCategory) -> Diagnostic {
    Diagnostic {
        message: Message { text: msg.into() },
        range: None,
        category,
    }
}
