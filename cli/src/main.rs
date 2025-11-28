use clap::{Arg, ArgMatches, Command};
use mainstage_core::{analyze_semantic_rules, ast::generate_ast_from_source};
use std::fs;

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
            )
            .arg(
                Arg::new("output")
                    .help("Specify the output file")
                    .short('o')
                    .long("output")
                    .value_parser(clap::value_parser!(String))
                    .value_name("FILE"),
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
            let out = sub_m.get_one::<String>("output");

            let script = mainstage_core::script::Script::new(std::path::PathBuf::from(file))
                .expect("Failed to load script file");

            // Properly handle the Result so we don't silently drop errors.
            let mut ast = match generate_ast_from_source(&script) {
                Ok(ast) => ast,
                Err(e) => {
                    // Print a helpful message and stop processing this command.
                    println!("Error generating AST: {}", e);
                    return;
                }
            };

            analyze_semantic_rules(&mut ast).map_err(|e| {
                e.iter().for_each(|f| println!("Semantic analysis error: {f}"));
            }).ok();

            if let Some(output_file) = out {
                fs::write(output_file, format!("{:#?}", ast)).expect("Failed to write output file");
            }

            if let Some(dump_stage) = sub_m.get_one::<String>("dump") {
                match dump_stage.as_str() {
                    "ast" => {
                        fs::write("dumped_ast.txt", format!("{:#?}", ast))
                            .expect("Failed to write dumped AST");
                    }
                    _ => {
                        println!("Unknown dump stage: {}", dump_stage);
                    }
                }
            }
        }
        Some(("run", sub_m)) => {
            let _file = sub_m.get_one::<String>("file").expect("required argument");

            if let Some(dump_stage) = sub_m.get_one::<String>("dump") {
                match dump_stage.as_str() {
                    "ast" => {}
                    _ => {
                        println!("Unknown dump stage: {}", dump_stage);
                    }
                }
            }
        }
        _ => {
            println!("No valid subcommand was used. Use --help for more information.");
        }
    }
}
