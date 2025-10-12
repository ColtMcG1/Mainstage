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

fn handle_build(matches: &ArgMatches, out: &mut mainstage_fmt_stdout_handler) {
    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        pipeline(path, out);
    } else {
        out.error("No path provided for build.").unwrap();
    }

    fn pipeline(path: &str, out: &mut mainstage_fmt_stdout_handler) {
        // Simulate stages of the build process
        let stages = vec![
            ("Parsing", "Parsing the script..."),
            ("Analyzing", "Analyzing the script..."),
            ("Compiling", "Compiling the script..."),
            ("Linking", "Linking the script..."),
        ];

        let mut collector = ReportCollector::new();
        let script = Script::new(path, &mut collector);
        let mut spnr = Spinner::new();
        let mut pb = Progress::new(stages.len());

        for (i, (stage, message)) in stages.iter().enumerate() {
            match *stage {
                "Parsing" => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    if let Ok(script) = parse_script(&script, &mut collector) {
                        out.info("Parsing completed successfully.")
                            .unwrap();
                    }
                    // simulate work
                }
                _ => {
                    // simulate work
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    out.message(message).unwrap();
                }
            }
            
            pb.advance();
            out.spinner_and_progress_with_message(&mut spnr, &mut pb, message, None)
                .unwrap();
        }

        if !collector.is_empty() {
            for report in collector.reports {
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
            }
        } else {
            out.message("No reports generated. Build successful.")
                .unwrap();
        }
    }
}

fn handle_run(matches: &ArgMatches, out: &mut mainstage_fmt_stdout_handler) {
    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        out.message(&format!("Running script at: {}", path))
            .unwrap();
        // Add run logic here
    } else {
        out.error("No path provided for run.").unwrap();
    }
}

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
fn teardown_output(out: &mut mainstage_fmt_stdout_handler) {
    out.bottom_border().unwrap();
    out.flush().unwrap();
}

fn main() {
    let mut out = setup_output();
    let matches = build_cli().try_get_matches().unwrap_or_else(|e| e.exit());

    match matches.subcommand() {
        Some(("build", sub_m)) => handle_build(sub_m, &mut out),
        Some(("run", sub_m)) => handle_run(sub_m, &mut out),
        _ => println!("No valid command provided. Use --help for more information."),
    }

    teardown_output(&mut out);
}
