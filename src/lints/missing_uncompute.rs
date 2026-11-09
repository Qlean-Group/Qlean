use crate::analysis::{check_uncompute, OperationTrace};
use crate::output::{Diagnostic, Severity};
use super::ancilla_leak::offset_to_line_col;

pub fn check(
    trace: &OperationTrace,
    file_name: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for lifetime in &trace.lifetimes {
        if lifetime.kind != crate::analysis::QubitKind::Fresh {
            continue;
        }
        let issues = check_uncompute(lifetime);
        for issue in issues {
            let (line, column) = offset_to_line_col(source, issue.gate_span.0);
            diagnostics.push(Diagnostic {
                file: file_name.to_string(),
                line,
                column,
                rule: "missing-uncompute".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "qubit '{}' has unreversed gate '{}' leaving state entangled before release",
                    issue.ancilla_name, issue.unreversed_gate
                ),
                qubit_ids: issue.involved_qubits,
            });
        }
    }
}
