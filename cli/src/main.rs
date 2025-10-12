/// A CLI tool for Mainstage
/// Author: Colton McGraw
/// Version: 0.1.0
/// License: TBD
/// Description: A command-line interface for Mainstage operations
/// Usage: mainstage [ACTION] path/to/script <args>
/// Actions:
///   -b, --build       Build the script
///   -r, --run         Run the script
/// Example: mainstage -b path/to/script --dump --verbose
use clap::{Arg, ArgMatches, Command};
use mainstage_core::*;
use std::io::{self};

mod output;

use output::*;

fn build_cli() -> Command {
    Command::new("mainstage")
        .version("0.1.0")
        .author("Colton McGraw")
        .about("A CLI tool for Mainstage")
        .subcommand(
            Command::new("build").about("Build the script").arg(
                Arg::new("path")
                    .index(1)
                    .required(true)
                    .help("Path to the script to build"),
            ),
        )
        .subcommand(
            Command::new("run")
                .about("Run the script")
                .arg(
                    Arg::new("path")
                        .index(1)
                        .required(true)
                        .help("Path to the script to run"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Perform a dry run without executing the script"),
                ),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Activate verbose mode")
                .num_args(0)
                .global(true),
        )
        .arg(
            Arg::new("dump")
                .short('d')
                .long("dump")
                .help("Dump the output of a stage")
                .value_name("stage")
                .num_args(1)
                .global(true),
        )
}

fn pipeline(path: &str, out: &mut mainstage_fmt_stdout_handler, collector: &mut ReportCollector) {
    // Simulate stages of the build process
    let stages = vec![
        ("Initializing", "Initializing build process..."),
        ("Expanding", "Expanding the script..."),
        ("Parsing", "Parsing the script..."),
        ("Analyzing", "Analyzing the script..."),
        ("Resolving", "Resolving dependencies..."),
        ("Graphing", "Creating the DAG..."),
        ("Planning", "Planning execution..."),
        ("Executing", "Executing the script..."),
    ];

    let script = Script::new(path, collector);
    let mut spnr = Spinner::new();
    let mut pb = Progress::new(stages.len());

    for (stage, message) in stages.iter() {
        match *stage {
            "Initializing" => {
                if collector.has_fatal() {
                    out.error("Initialization failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                } else {
                    out.success("Initialization completed successfully.").unwrap();
                }
            }
            "Expanding" => {
                if let Ok(script) = expand_script(&script, collector) {
                    out.success("Expansion completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Expansion failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Parsing" => {
                if let Ok(script) = parse_script(&script, collector) {
                    out.success("Parsing completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Parsing failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Analyzing" => {
                if let Ok(script) = analyze_script(&script, collector) {
                    out.success("Analysis completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Analysis failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Resolving" => {
                if let Ok(script) = resolve_script(&script, collector) {
                    out.success("Resolution completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Resolution failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Graphing" => {
                if let Ok(script) = make_script_dag(&script, collector) {
                    out.success("DAG creation completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("DAG creation failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Planning" => {
                if let Ok(script) = plan_script(&script, collector) {
                    out.success("Planning completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Planning failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            "Executing" => {
                if let Ok(script) = execute_script(&script, collector) {
                    out.success("Execution completed successfully.").unwrap();
                } else if collector.has_fatal() {
                    out.error("Execution failed with fatal errors. Aborting build.")
                        .unwrap();
                    break;
                }
            }
            _ => {
                out.warning(&format!("Unknown stage: {}", stage)).unwrap();
            }
        }

        pb.advance();
        out.spinner_and_progress_with_message(&mut spnr, &mut pb, message, None)
            .unwrap();
    }

    out.hr().unwrap();
    
    if !collector.is_empty() {
        out.subtitle(&format!("Reports ({})", collector.reports.len())).unwrap();
        out.hr().unwrap();
        collector.reports.clone().iter().for_each(|report| {
            match report.severity {
                Severity::Info => out.message(&format!("Info: {}", report.message)).unwrap(),
                Severity::Warning => out
                    .warning(&format!("Warning: {}", report.message))
                    .unwrap(),
                Severity::Error => out.error(&format!("Error: {}", report.message)).unwrap(),
                Severity::Fatal => {
                    out.error(&format!("Fatal Error: {}", report.message))
                        .unwrap();
                    return;
                }
            }
        });
    } else {
        out.message("No reports generated. Build successful.")
            .unwrap();
    }
}

fn handle_build(
    matches: &ArgMatches,
    out: &mut mainstage_fmt_stdout_handler,
    collector: &mut ReportCollector,
) {
    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        pipeline(path, out, collector);
    } else {
        out.error("No path provided for build.").unwrap();
    }
}

fn handle_run(matches: &ArgMatches, out: &mut mainstage_fmt_stdout_handler, collector: &mut ReportCollector) {
    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        out.message(&format!("Running script at: {}", path))
            .unwrap();
        // Add run logic here
    } else {
        out.error("No path provided for run.").unwrap();
    }
}

/// Setup formatted output handler
fn setup_output() -> mainstage_fmt_stdout_handler {
    let width = 80; // or any desired width
    let mut out =
        FormattedOutputHandler::new(io::stdout(), width).with_newline_on_task_complete(false);
    out.top_border().unwrap();
    let title_style = out.get_formatting().title.clone();
    let message_style = out.get_formatting().info.clone();
    out.line(
        &format!("Mainstage{:>5}v{}", " ", env!("CARGO_PKG_VERSION")),
        Some(&title_style),
        true,
    )
    .unwrap();
    out.hr().unwrap();
    out.line("Building the script...", Some(&message_style), true)
        .unwrap();
    out.hr().unwrap();
    out
}
/// Teardown formatted output handler
fn teardown_output(out: &mut mainstage_fmt_stdout_handler) {
    out.bottom_border().unwrap();
    out.flush().unwrap();
}

fn main() {
    let mut collector = ReportCollector::new();
    let mut out = setup_output();
    let matches = build_cli().try_get_matches().unwrap_or_else(|e| e.exit());

    match matches.subcommand() {
        Some(("build", sub_m)) => handle_build(sub_m, &mut out, &mut collector),
        Some(("run", sub_m)) => handle_run(sub_m, &mut out, &mut collector),
        _ => println!("No valid command provided. Use --help for more information."),
    }

    teardown_output(&mut out);
}
