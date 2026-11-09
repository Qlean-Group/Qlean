use crate::analysis::OperationTrace;
use crate::output::{Diagnostic, Severity};
use super::ancilla_leak::offset_to_line_col;

pub fn check(
    trace: &OperationTrace,
    file_name: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for event in &trace.entanglements {
        let (line, column) = offset_to_line_col(source, event.condition_span.0);
        diagnostics.push(Diagnostic {
            file: file_name.to_string(),
            line,
            column,
            rule: "mid-circuit-entanglement".to_string(),
            severity: Severity::Warning,
            message: format!(
                "measurement of qubit '{}' conditionally controls operations on qubit '{}'",
                event.source_qubit, event.target_qubit
            ),
            qubit_ids: vec![event.source_qubit.clone(), event.target_qubit.clone()],
        });
    }
}

