//! MaingStage CLI
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-14
//! description: A CLI for analyzing and reporting on script files using MainStage core functionalities.

use clap::{Arg, ArgMatches, Command};
use mainstage_core::reports::accumulator::*;
use std::path::Path;

// Import your actual core library (adjust the path as needed)
use mainstage_core::*;

/// This is the main entry point for the MainStage CLI application.
/// It sets up the command-line interface, parses arguments, and dispatches commands.
fn main() {
    let cli = Command::new("MainStage CLI")
        .version("0.1.0")
        .author("Colton McGraw <https://github.com/ColtMcG1>")
        .about("A CLI for MainStage");

    let cli = setup_cli(cli);
    let matches = cli.get_matches();
    dispatch_commands(&matches);
}

/// Sets up the CLI with subcommands and arguments.
/// This function configures the command-line interface using the `clap` crate.
/// It defines subcommands for analyzing scripts and generating reports.
fn setup_cli(cli: Command) -> Command {
    cli.subcommand(
        Command::new("build")
            .about("Build the specified script file")
            .arg(
                Arg::new("file")
                    .help("The script file to build")
                    .required(true)
                    .index(1),
            )
            .arg(
                Arg::new("dump")
                    .help("Specify the dump stage")
                    .short('d')
                    .long("dump")
                    .value_parser(clap::value_parser!(String))
                    .value_name("STAGE"),
            ),
    )
    .subcommand(
        Command::new("run")
            .about("Run a script file")
            .arg(
                Arg::new("file")
                    .help("The script file to run")
                    .required(true)
                    .index(1),
            )
            .arg(
                Arg::new("dump")
                    .help("Specify the dump stage")
                    .short('d')
                    .long("dump")
                    .value_parser(clap::value_parser!(String))
                    .value_name("STAGE"),
            ),
    )
}

/// Dispatches the command based on the parsed arguments.
/// This function matches the subcommand used and calls the appropriate handler.
fn dispatch_commands(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("build", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let dump = sub_m.get_one::<String>("dump");
            let path = Path::new(file);
            let mut pipeline = Pipeline::new();
            let dump_option = dump.map(|stage| Dump {
                stage: stage.clone(),
            });
            pipeline.run(path, dump_option);
            show_reports(&Accumulator::get_instance().get_reports());
        }
        Some(("run", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let dump = sub_m.get_one::<String>("dump");
            let path = Path::new(file);
            let mut pipeline = Pipeline::new();
            let dump_option = dump.map(|stage| Dump {
                stage: stage.clone(),
            });
            pipeline.run(path, dump_option);
            show_reports(&Accumulator::get_instance().get_reports());
        }
        _ => {
            println!("No valid subcommand was used. Use --help for more information.");
        }
    }
}

/// Converts the reports in the accumulator to a human-readable format and prints them.
/// This function iterates over the reports and formats them for display in the CLI.
// Import Accumulator from its correct location at the top of the file:
fn show_reports(reports: &Vec<mainstage_core::reports::Report>) {
    for report in reports.iter() {
        let timestamp = report.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let styled_time = console::style(format!("[{}]", timestamp)).dim();
        let styled_level = match report.level {
            reports::Level::Critical => console::style("[CRITICAL]").red().bold(),
            reports::Level::Error => console::style("[ERROR]").red(),
            reports::Level::Warning => console::style("[WARNING]").yellow(),
            reports::Level::Info => console::style("[INFO]").blue(),
        };
        let styled_report = format!(
            "{} (Issuer: {})",
            report.message,
            report
                .issuer
                .as_ref()
                .map_or("Unknown".to_string(), |issuer| issuer.clone())
        );
        println!(
            "{} {} {} {}",
            styled_time,
            styled_level,
            styled_report,
            if let Some(location) = &report.location {
                format!(" at {}", location)
            } else {
                "".to_string()
            }
        );
    }
}
