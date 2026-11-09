pub mod analysis;
pub mod lints;
pub mod output;
pub mod wasm;

use analysis::{LifetimeTracker, OperationTrace};
use output::Diagnostic;
use qsc::PackageType;
use qsc_data_structures::{
    language_features::LanguageFeatures, source::SourceMap, target::TargetCapabilityFlags,
};
use qsc_frontend::compile::{CompileUnit, PackageStore};
use qsc_hir::hir::{CallableKind, ItemKind};

pub fn compile_source(
    name: &str,
    source: &str,
) -> Result<(CompileUnit, PackageStore), Vec<String>> {
    let capabilities = TargetCapabilityFlags::all();
    let (std_id, store) = qsc::compile::package_store_with_stdlib(capabilities);
    let dependencies = vec![(std_id, None)];
    let sources = SourceMap::new([(name.into(), source.into())], None);
    let (unit, errors) = qsc::compile::compile(
        &store,
        &dependencies,
        sources,
        PackageType::Lib,
        capabilities,
        LanguageFeatures::default(),
    );

    if !errors.is_empty() {
        let err_msgs = errors.iter().map(|e| e.to_string()).collect();
        return Err(err_msgs);
    }

    Ok((unit, store))
}

pub fn parse_operations(name: &str, source: &str) -> Result<Vec<String>, Vec<String>> {
    let (unit, _) = compile_source(name, source)?;
    let mut ops = Vec::new();
    for (_, item) in &unit.package.items {
        if let ItemKind::Callable(decl) = &item.kind {
            let kind = match decl.kind {
                CallableKind::Function => "function",
                CallableKind::Operation => "operation",
            };
            ops.push(format!("{} {}", kind, decl.name.name));
        }
    }
    Ok(ops)
}

pub fn analyze_source(
    name: &str,
    source: &str,
) -> Result<Vec<OperationTrace>, Vec<String>> {
    let (unit, store) = compile_source(name, source)?;
    let mut traces = Vec::new();
    for (_, item) in &unit.package.items {
        if let ItemKind::Callable(decl) = &item.kind {
            if decl.kind == CallableKind::Operation {
                let tracker = LifetimeTracker::new(&store, &unit);
                traces.push(tracker.trace_operation(decl));
            }
        }
    }
    Ok(traces)
}

pub fn lint_source(name: &str, source: &str) -> Vec<Diagnostic> {
    lint_source_with_options(name, source, false)
}

pub fn lint_source_with_options(name: &str, source: &str, include_ast: bool) -> Vec<Diagnostic> {
    let (unit, store) = match compile_source(name, source) {
        Ok(res) => res,
        Err(errors) => {
            return errors
                .into_iter()
                .map(|msg| Diagnostic {
                    file: name.to_string(),
                    line: 1,
                    column: 1,
                    rule: "compilation-error".to_string(),
                    severity: output::Severity::Error,
                    message: msg,
                    qubit_ids: Vec::new(),
                })
                .collect();
        }
    };

    let mut traces = Vec::new();
    for (_, item) in &unit.package.items {
        if let ItemKind::Callable(decl) = &item.kind {
            if decl.kind == CallableKind::Operation {
                let tracker = LifetimeTracker::new(&store, &unit);
                traces.push(tracker.trace_operation(decl));
            }
        }
    }

    let mut diagnostics = lints::run_all_lints(&traces, name, source);

    if include_ast {
        let qsc_lints = qsc::linter::run_lints(&store, &unit, None);
        for qsc_lint in qsc_lints {
            let (line, column) = if let Some(src) = unit.sources.find_by_offset(qsc_lint.span.lo) {
                let rel_offset = (qsc_lint.span.lo.saturating_sub(src.offset)) as usize;
                crate::lints::ancilla_leak::offset_to_line_col(&src.contents, rel_offset)
            } else {
                let rel_offset = qsc_lint.span.lo as usize;
                crate::lints::ancilla_leak::offset_to_line_col(source, rel_offset)
            };

            let severity = match qsc_lint.level {
                qsc::linter::LintLevel::Allow => continue,
                qsc::linter::LintLevel::Warn | qsc::linter::LintLevel::ForceWarn => {
                    output::Severity::Warning
                }
                qsc::linter::LintLevel::Error | qsc::linter::LintLevel::ForceError => {
                    output::Severity::Error
                }
            };

            let rule_name = serde_json::to_value(qsc_lint.kind)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("{:?}", qsc_lint.kind));

            diagnostics.push(Diagnostic {
                file: name.to_string(),
                line,
                column,
                rule: rule_name,
                severity,
                message: qsc_lint.message.to_string(),
                qubit_ids: Vec::new(),
            });
        }
    }

    diagnostics
}
