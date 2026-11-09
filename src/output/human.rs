use super::Diagnostic;

pub fn format_human(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        out.push_str(&format!(
            "{}:{}:{}: {}: {} [{}]\n",
            d.file, d.line, d.column, d.severity, d.message, d.rule
        ));
    }
    out
}

