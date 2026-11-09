use crate::analysis::OperationTrace;
use crate::output::{Diagnostic, Severity};

pub fn check(
    trace: &OperationTrace,
    file_name: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for lifetime in &trace.lifetimes {
        if lifetime.has_leak {
            let (span_lo, _) = lifetime
                .release_span
                .filter(|(lo, hi)| *lo > 0 && *hi > *lo)
                .unwrap_or(lifetime.qubit.span);

            let (line, column) = offset_to_line_col(source, span_lo);

            diagnostics.push(Diagnostic {
                file: file_name.to_string(),
                line,
                column,
                rule: "ancilla-leak".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "qubit '{}' allocated but never reset before release",
                    lifetime.qubit.name
                ),
                qubit_ids: vec![lifetime.qubit.name.clone()],
            });
        }
    }
}

pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

