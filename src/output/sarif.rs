use super::Diagnostic;
use serde_json::json;

pub fn format_sarif(diagnostics: &[Diagnostic]) -> String {
    let results: Vec<_> = diagnostics
        .iter()
        .map(|d| {
            let level = match d.severity {
                crate::output::Severity::Error => "error",
                crate::output::Severity::Warning => "warning",
                crate::output::Severity::Info => "note",
            };
            json!({
                "ruleId": d.rule,
                "level": level,
                "message": {
                    "text": d.message
                },
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": d.file
                            },
                            "region": {
                                "startLine": d.line,
                                "startColumn": d.column
                            }
                        }
                    }
                ]
            })
        })
        .collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "Qlean",
                        "version": "0.1.0",
                        "informationUri": "https://github.com/microsoft/qsharp"
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".to_string())
}

