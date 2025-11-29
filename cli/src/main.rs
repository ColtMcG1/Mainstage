use clap::{Arg, ArgMatches, Command};
use mainstage_core::{analyze_acyclic_rules, analyze_semantic_rules, ast::generate_ast_from_source};
use std::fs;

mod disassembler;

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
                Arg::new("optimize")
                    .help("Enable IR optimization")
                    .short('O')
                    .long("optimize")
                    .action(clap::ArgAction::SetTrue),
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
                Arg::new("optimize")
                    .help("Enable IR optimization")
                    .short('O')
                    .long("optimize")
                    .action(clap::ArgAction::SetTrue),
            )
    )
    .subcommand(
        Command::new("disasm")
        .about("Disassemble a .msx file")
        .arg(
            Arg::new("file")
                .help("The .msx file to disassemble")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .help("Specify the output file for disassembly")
                .short('o')
                .long("output")
                .value_parser(clap::value_parser!(String))
                .value_name("FILE")
                .required(true),
        )
    )
}

/// Dispatches the command based on the parsed arguments.
/// This function matches the subcommand used and calls the appropriate handler.
fn dispatch_commands(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("build", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let out = sub_m.get_one::<String>("output");
            let optimize = sub_m.get_flag("optimize");

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

            let (entry, analysis) = match analyze_semantic_rules(&mut ast) {
                Ok((name, analysis)) => (name, analysis),
                Err(diags) => {
                    diags.iter().for_each(|d| println!("Semantic analysis error: {d}"));
                    return;
                }
            };

            if let Err(e) = analyze_acyclic_rules(&ast) {
                println!("Acyclic analysis error: {}", e);
                return;
            }

            let ir_module = mainstage_core::ir::lower_ast_to_ir(&ast, &entry, optimize, Some(&analysis));

            let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);

            if let Some(output_file) = out {
                fs::write(output_file.to_owned() + ".msx", &bytecode).expect("Failed to write output file");
            }

            if let Some(dump_stage) = sub_m.get_one::<String>("dump") {
                match dump_stage.as_str() {
                    "ast" => {
                        fs::write("dumped_ast.txt", format!("{:#?}", ast))
                            .expect("Failed to write dumped AST");
                    }
                    "ir" => {
                        fs::write("dumped_ir.txt", format!("{}", ir_module))
                            .expect("Failed to write dumped IR");
                    }
                    _ => {
                        println!("Unknown dump stage: {}", dump_stage);
                    }
                }
            }
        }
        Some(("run", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let optimize = sub_m.get_flag("optimize");

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

            let (entry, _analysis) = match analyze_semantic_rules(&mut ast) {
                Ok((name, analysis)) => (name, analysis),
                Err(diags) => {
                    diags.iter().for_each(|d| println!("Semantic analysis error: {d}"));
                    return;
                }
            };

            if let Err(e) = analyze_acyclic_rules(&ast) {
                println!("Acyclic analysis error: {}", e);
                return;
            }

            let ir_module = mainstage_core::ir::lower_ast_to_ir(&ast, &entry, optimize, Some(&_analysis));

            let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);
            // Run the bytecode in the VM
            match mainstage_core::run_bytecode(&bytecode) {
                Ok(()) => {}
                Err(e) => println!("Runtime error: {}", e),
            }
        }
        Some(("disasm", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let output_file = sub_m.get_one::<String>("output").expect("required argument");

            let bytecode = fs::read(file).expect("Failed to read .msx file");

            match disassembler::disassemble(&bytecode) {
                Ok(f) => {
                    if let Err(e) = fs::write(output_file, f) {
                        println!("Failed to write disassembly output file: {}", e);
                    }
                }
                Err(e) => {
                    println!("Failed to disassemble bytecode: {}", e);
                }
            }
        }
        _ => {
            println!("No valid subcommand was used. Use --help for more information.");
        }
    }
}
