use std::fs;
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand, ValueEnum};
use qlean::output::human::format_human;
use qlean::output::json::format_json;
use qlean::output::sarif::format_sarif;
use qlean::{analyze_source, lint_source_with_options, parse_operations};

#[derive(Parser, Debug)]
#[command(name = "qlean")]
#[command(version = "0.1.0")]
#[command(about = "Semantic static analyzer for Q#")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Human,
    Json,
    Sarif,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Parse {
        path: PathBuf,
    },
    Trace {
        path: PathBuf,
    },
    Lint {
        path: PathBuf,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        #[arg(long, help = "Include Microsoft pattern and syntax lints")]
        include_ast: bool,
    },
}

fn collect_qs_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(collect_qs_files(&p));
            } else if p.extension().is_some_and(|ext| ext == "qs") {
                files.push(p);
            }
        }
    }
    files
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => {
            let files = collect_qs_files(&path);
            if files.is_empty() {
                eprintln!("No .qs files found at {:?}", path);
                std::process::exit(1);
            }
            for file in files {
                let content = match fs::read_to_string(&file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {:?}: {}", file, e);
                        continue;
                    }
                };
                let file_str = file.to_string_lossy();
                match parse_operations(&file_str, &content) {
                    Ok(ops) => {
                        println!("Operations in {}:", file_str);
                        for op in ops {
                            println!("  {}", op);
                        }
                    }
                    Err(errs) => {
                        eprintln!("Errors parsing {}:", file_str);
                        for err in errs {
                            eprintln!("  {}", err);
                        }
                    }
                }
            }
        }
        Commands::Trace { path } => {
            let files = collect_qs_files(&path);
            if files.is_empty() {
                eprintln!("No .qs files found at {:?}", path);
                std::process::exit(1);
            }
            for file in files {
                let content = match fs::read_to_string(&file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {:?}: {}", file, e);
                        continue;
                    }
                };
                let file_str = file.to_string_lossy();
                match analyze_source(&file_str, &content) {
                    Ok(traces) => {
                        println!("Trace for {}:", file_str);
                        for trace in traces {
                            println!("  Operation: {}", trace.operation_name);
                            for lifetime in &trace.lifetimes {
                                println!(
                                    "    Qubit '{}' ({}) - Final State: {} - Has Leak: {}",
                                    lifetime.qubit.name,
                                    lifetime.kind,
                                    lifetime.state,
                                    lifetime.has_leak
                                );
                                for t in &lifetime.transitions {
                                    println!("      Step {}: {} ({})", t.step, t.kind, t.details);
                                }
                            }
                            for ent in &trace.entanglements {
                                println!(
                                    "    Entanglement: '{}' conditionally controlling '{}'",
                                    ent.source_qubit, ent.target_qubit
                                );
                            }
                        }
                    }
                    Err(errs) => {
                        eprintln!("Errors analyzing {}:", file_str);
                        for err in errs {
                            eprintln!("  {}", err);
                        }
                    }
                }
            }
        }
        Commands::Lint {
            path,
            format,
            include_ast,
        } => {
            let files = collect_qs_files(&path);
            if files.is_empty() {
                eprintln!("No .qs files found at {:?}", path);
                std::process::exit(1);
            }
            let mut all_diags = Vec::new();
            for file in files {
                let content = match fs::read_to_string(&file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {:?}: {}", file, e);
                        continue;
                    }
                };
                let file_str = file.to_string_lossy();
                let diags = lint_source_with_options(&file_str, &content, include_ast);
                all_diags.extend(diags);
            }

            match format {
                OutputFormat::Human => {
                    print!("{}", format_human(&all_diags));
                }
                OutputFormat::Json => {
                    println!("{}", format_json(&all_diags));
                }
                OutputFormat::Sarif => {
                    println!("{}", format_sarif(&all_diags));
                }
            }

            if all_diags.iter().any(|d| d.severity == qlean::output::Severity::Error) {
                std::process::exit(1);
            }
        }
    }
}
