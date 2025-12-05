//! file: cli/src/main.rs
//! description: command-line interface for MainStage.
//!
//! This binary provides user-facing commands to build, analyze and run
//! MainStage scripts. It wires together the `mainstage_core` APIs, performs
//! plugin discovery, and exposes subcommands for common developer workflows.
//!
use clap::{Arg, ArgMatches, Command};
use console::style;
use log::{Level, error, info, warn};
use mainstage_core::{
    VM, analyze_acyclic_rules, analyze_semantic_rules, ast::generate_ast_from_source,
};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

mod disassembler;

fn main() {
    // Initialize logger with a clean, human-friendly format and colored level tags.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let lvl = match record.level() {
                Level::Error => style("error").red().bold(),
                Level::Warn => style("warn").yellow().bold(),
                Level::Info => style("info").green().bold(),
                Level::Debug => style("debug").cyan(),
                Level::Trace => style("trace").magenta(),
            };
            writeln!(buf, "{}: {}", lvl, record.args())
        })
        .init();

    let cli = Command::new("MainStage")
        .version("0.1.0")
        .author("Colton McGraw <https://github.com/ColtMcG1>")
        .about("A CLI for MainStage");

    let cli = setup_cli(cli).arg(
        Arg::new("plugin-dir")
            .help("Directory to load plugins from")
            .short('P')
            .long("plugin-dir")
            .value_parser(clap::value_parser!(String))
            .value_name("DIR")
            .global(true),
    );

    let matches = cli.get_matches();

    // VM plugin discovery (CLI may override the directory)
    let mut vm = VM::new(vec![]);
    let plugin_dir: Option<PathBuf> = matches
        .get_one::<String>("plugin-dir")
        .map(|s| PathBuf::from(s));
    match vm.discover_plugins(plugin_dir.as_ref()) {
        Ok(n) => info!("Discovered {} plugin manifest(s)", n),
        Err(e) => error!("Plugin discovery failed: {}", e),
    }

    // Clone descriptors map for analyzer usage during CLI commands.
    let manifests_map = vm.plugin_descriptors();

    dispatch_commands(&matches, &manifests_map);
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
            .arg(
                Arg::new("trace")
                    .help("Enable tracing of script execution")
                    .short('t')
                    .long("trace")
                    .action(clap::ArgAction::SetTrue),
            ),
    )
    .subcommand(
        Command::new("inspect")
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
                    .value_name("FILE"),
            ),
    )
}

/// Dispatches the command based on the parsed arguments.
/// This function matches the subcommand used and calls the appropriate handler.
fn dispatch_commands(
    matches: &ArgMatches,
    manifests: &std::collections::HashMap<String, mainstage_core::vm::plugin::PluginDescriptor>,
) {
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
                    error!("Error generating AST: {}", e);
                    return;
                }
            };

            let (entry, analysis) = match analyze_semantic_rules(&mut ast, Some(manifests)) {
                Ok((name, analysis)) => (name, analysis),
                Err(diags) => {
                    diags
                        .iter()
                        .for_each(|d| error!("Semantic analysis error: {d}"));
                    return;
                }
            };

            if let Err(e) = analyze_acyclic_rules(&ast) {
                error!("Acyclic analysis error: {}", e);
                return;
            }

            let ir_module =
                mainstage_core::ir::lower_ast_to_ir(&ast, &entry, optimize, Some(&analysis));

            let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);

            if let Some(output_file) = out {
                fs::write(output_file.to_owned() + ".msx", &bytecode)
                    .expect("Failed to write output file");
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
                        error!("Unknown dump stage: {}", dump_stage);
                    }
                }
            }
        }

        Some(("run", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let optimize = sub_m.get_flag("optimize");
            let trace = sub_m.get_flag("trace");

            let script = mainstage_core::script::Script::new(std::path::PathBuf::from(file))
                .expect("Failed to load script file");

            // Properly handle the Result so we don't silently drop errors.
            let mut ast = match generate_ast_from_source(&script) {
                Ok(ast) => ast,
                Err(e) => {
                    // Print a helpful message and stop processing this command.
                    error!("Error generating AST: {}", e);
                    return;
                }
            };

            let (entry, analysis) = match analyze_semantic_rules(&mut ast, Some(manifests)) {
                Ok((name, analysis)) => (name, analysis),
                Err(diags) => {
                    diags
                        .iter()
                        .for_each(|d| error!("Semantic analysis error: {d}"));
                    return;
                }
            };

            if let Err(e) = analyze_acyclic_rules(&ast) {
                error!("Acyclic analysis error: {}", e);
                return;
            }

            let ir_module =
                mainstage_core::ir::lower_ast_to_ir(&ast, &entry, optimize, Some(&analysis));

            let bytecode = mainstage_core::ir::emit_bytecode(&ir_module);
            // Run the bytecode in the VM. We register external plugin
            // executables before switching the process working directory so
            // relative manifest paths are resolved against the original CWD.
            let orig_cwd = std::env::current_dir().ok();

            // Create the VM now so we can register runtime plugin instances
            // against it before we change the CWD to the script location.
            let mut run_vm = mainstage_core::VM::new(bytecode);

            // Collect imports from the source text as a fallback: alias -> module name
            let mut import_aliases: Vec<(String, String)> = Vec::new();
            // Use the already-loaded script content instead of re-reading the file.
            let src_text = script.display_content().to_string();
            for line in src_text.lines() {
                let s = line.trim();
                if s.starts_with("import ") {
                    // very small parser for: import "mod" as alias;
                    // tolerant to spacing
                    if let Some(rest) = s.strip_prefix("import ") {
                        let parts: Vec<&str> = rest.split_whitespace().collect();
                        if parts.len() >= 3 && parts[1] == "as" {
                            let module = parts[0].trim().trim_matches('"').to_string();
                            let alias = parts[2].trim().trim_end_matches(';').to_string();
                            import_aliases.push((alias, module));
                        }
                    }
                }
            }

            // Register external plugin executables under each alias when present
            for (alias, mod_name) in import_aliases.into_iter() {
                if let Some(desc) = manifests.get(&mod_name) {
                    if let Some(dir) = &desc.path {
                        // The manifest can include a `path` attribute which may point
                        // to the plugin executable (file) or a directory containing
                        // it. Prefer this field when present.
                        let entry = desc
                            .manifest
                            .entry
                            .clone()
                            .unwrap_or_else(|| desc.manifest.name.clone());

                        // canonical base directory for resolving manifest-relative paths
                        let manifest_dir = dir.clone();

                        // If manifest.path (the field inside the JSON) is set, try it first.
                        let mut exe_candidates: Vec<std::path::PathBuf> = Vec::new();
                        if !desc.manifest.path.trim().is_empty() {
                            let mp = std::path::PathBuf::from(desc.manifest.path.clone());
                            let mp_resolved = if mp.is_absolute() {
                                mp
                            } else {
                                // resolve relative manifest.path against the manifest directory
                                manifest_dir.join(mp)
                            };
                            exe_candidates.push(mp_resolved.clone());
                            // if it looks like a directory, append the entry name
                            if mp_resolved.is_dir() {
                                exe_candidates.push(mp_resolved.join(&entry));
                            }
                        }

                        // Fallback: prefer executable sitting next to manifest
                        let next_to_manifest = manifest_dir.join(&entry);
                        exe_candidates.push(next_to_manifest.clone());

                        // Also try with .exe suffix and typical cargo target locations
                        let mut with_exe = next_to_manifest.clone();
                        with_exe.set_extension("exe");
                        exe_candidates.push(with_exe.clone());
                        let crate_root = manifest_dir
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or(manifest_dir.clone());
                        let cand_debug = crate_root.join("target").join("debug").join(&entry);
                        exe_candidates.push(cand_debug.clone());
                        let mut cand_debug_exe = cand_debug.clone();
                        cand_debug_exe.set_extension("exe");
                        exe_candidates.push(cand_debug_exe.clone());
                        let cand_rel = crate_root.join("target").join("release").join(&entry);
                        exe_candidates.push(cand_rel.clone());
                        let mut cand_rel_exe = cand_rel.clone();
                        cand_rel_exe.set_extension("exe");
                        exe_candidates.push(cand_rel_exe.clone());

                        // Pick the first candidate that exists
                        let mut found: Option<std::path::PathBuf> = None;
                        for c in exe_candidates.into_iter() {
                            if c.exists() {
                                found = Some(c);
                                break;
                            }
                        }

                        if let Some(exe) = found {
                            // Try to canonicalize to an absolute path so later
                            // spawns are not affected by CWD changes.
                            let exe_abs = std::fs::canonicalize(&exe).unwrap_or(exe.clone());
                            let ep = mainstage_core::vm::external::ExternalPlugin::new(
                                alias.clone(),
                                exe_abs,
                            );
                            run_vm.register_plugin(std::sync::Arc::new(ep));
                        } else {
                            warn!(
                                "could not locate executable for plugin module '{}' at expected path(s)",
                                mod_name
                            );
                        }
                    } else {
                        warn!(
                            "no path specified in manifest for imported module '{}'",
                            mod_name
                        );
                    }
                } else {
                    warn!(
                        "no plugin descriptor found for imported module '{}'",
                        mod_name
                    );
                }
            }
            // Now that plugin registration is complete, switch the process
            // working directory to the script folder so host functions like
            // `read` and glob-based resolution work relative to the script.
            if let Some(parent) = script.path.parent() {
                if let Err(e) = std::env::set_current_dir(parent) {
                    warn!("failed to set working dir to {:?}: {}", parent, e);
                }
            }

            match run_vm.run(trace) {
                Ok(()) => {}
                Err(e) => {
                    error!("{}", e.lines().collect::<Vec<&str>>().join("\n\t"));
                }
            }

            // Restore original working directory if available
            if let Some(orig) = orig_cwd {
                let _ = std::env::set_current_dir(orig);
            }
        }
        Some(("disasm", sub_m)) => {
            let file = sub_m.get_one::<String>("file").expect("required argument");
            let output_file = sub_m.get_one::<String>("output");

            let bytecode = fs::read(file).expect("Failed to read .msx file");

            match disassembler::disassemble(&bytecode) {
                Ok(f) => {
                    if let Some(output_file) = output_file {
                        if let Err(e) = fs::write(output_file, f) {
                            error!("Failed to write disassembly output file: {}", e);
                        }
                    } else {
                        println!("{}", f);
                    }
                }
                Err(e) => {
                    error!("Failed to disassemble bytecode: {}", e);
                }
            }
        }
        _ => {
            error!("No valid subcommand was used. Use --help for more information.");
        }
    }
}
