pub mod ancilla_leak;
pub mod entanglement;
pub mod missing_uncompute;

use crate::analysis::OperationTrace;
use crate::output::Diagnostic;

pub fn run_all_lints(
    traces: &[OperationTrace],
    file_name: &str,
    source: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for trace in traces {
        ancilla_leak::check(trace, file_name, source, &mut diagnostics);
        missing_uncompute::check(trace, file_name, source, &mut diagnostics);
        entanglement::check(trace, file_name, source, &mut diagnostics);
    }
    diagnostics
}

