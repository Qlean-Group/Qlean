use super::lifetime::{GateApplication, QubitLifetime, TransitionKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncomputeIssue {
    pub ancilla_name: String,
    pub unreversed_gate: String,
    pub gate_span: (usize, usize),
    pub involved_qubits: Vec<String>,
}

pub fn check_uncompute(lifetime: &QubitLifetime) -> Vec<UncomputeIssue> {
    let was_measured = lifetime
        .transitions
        .iter()
        .any(|t| matches!(t.kind, TransitionKind::Measurement(_)));

    if was_measured {
        return Vec::new();
    }

    let mut stack: Vec<&GateApplication> = Vec::new();

    for gate in &lifetime.gate_history {
        if let Some(top) = stack.last() {
            if are_inverses(top, gate) {
                stack.pop();
                continue;
            }
        }
        stack.push(gate);
    }

    let mut issues = Vec::new();
    for remaining in stack {
        if remaining.qubits.len() > 1 || remaining.gate_name.contains("Controlled") {
            issues.push(UncomputeIssue {
                ancilla_name: lifetime.qubit.name.clone(),
                unreversed_gate: remaining.gate_name.clone(),
                gate_span: remaining.span,
                involved_qubits: vec![lifetime.qubit.name.clone()],
            });
        }
    }

    issues
}

fn are_inverses(g1: &GateApplication, g2: &GateApplication) -> bool {
    if g1.qubits != g2.qubits {
        return false;
    }

    if is_self_inverse(&g1.gate_name) && g1.gate_name == g2.gate_name && !g1.is_adjoint && !g2.is_adjoint {
        return true;
    }

    if g1.gate_name == g2.gate_name && g1.is_adjoint != g2.is_adjoint {
        return true;
    }

    false
}

fn is_self_inverse(gate_name: &str) -> bool {
    matches!(
        gate_name,
        "CNOT"
            | "CX"
            | "CCNOT"
            | "X"
            | "Y"
            | "Z"
            | "H"
            | "SWAP"
            | "Swap"
            | "Controlled X"
            | "Controlled Z"
    )
}
