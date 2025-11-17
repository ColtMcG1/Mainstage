//! ./lib.rs
//!
//! Core library for the Mainstage project.
//! This library provides the core functionality for the Mainstage project, including report management and script handling.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

use crate::acyclic::*;
use crate::reports::*;
use crate::scripts::*;
use crate::semantic::*;

#[macro_use]
pub mod reports;

pub mod acyclic;
pub mod codegen;
pub mod parser;
pub mod runtime;
pub mod scripts;
pub mod semantic;

use std::rc::Rc;

/// Represents a dump stage in the pipeline.
/// This struct is used to specify at which stage of the pipeline a dump should occur.
/// # Examples
/// ```
/// let dump = Dump { stage: "loader".into() };
/// ```
pub struct Dump {
    pub stage: String,
}

/// The main pipeline struct that orchestrates the processing of scripts and accumulation of reports.
/// This struct manages the flow of data through various stages, generating reports as needed.
/// It utilizes the `Accumulator` to collect reports generated during processing.
/// # Examples
/// ```
/// let mut pipeline = Pipeline::new();
/// pipeline.run(std::path::Path::new("example.ms"));
/// let reports = pipeline.get_reports();
/// ```
pub struct Pipeline {
    /// The script being processed in the pipeline.
    script: Option<script::Script>,
    /// The AST parser used for parsing scripts.
    parser: Option<parser::AstParser>,
    /// The semantic analyzer used for semantic analysis.
    semantic: Option<SemanticAnalyzer<'static>>,
    /// The acyclic analyzer used for acyclic analysis.
    acyclic: Option<DirectedAcyclicGraphAnalyzer>,
    /// The IR module generated during code generation.
    ir: Option<codegen::IRProgram>,
}

impl Pipeline {
    /// Creates a new `Pipeline` instance.
    /// This initializes the report accumulator and sets the script to `None`.
    /// # Examples
    /// ```
    /// let pipeline = Pipeline::new();
    /// ```
    pub fn new() -> Self {
        Self {
            script: None,
            parser: None,
            semantic: None,
            acyclic: None,
            ir: None,
        }
    }

    /// Runs the pipeline on the provided script path.
    /// This function processes the script, analyzes it, and generates reports.
    /// # Arguments
    /// * `path` - The path to the script file to be processed.
    /// * `dump` - An optional `Dump` struct specifying at which stage to dump output.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.run(std::path::Path::new("example.js"));
    /// let reports = pipeline.get_reports();
    /// ```
    pub fn run(&mut self, path: &std::path::Path, dump: Option<Dump>) {
        // Load the script, and generate reports.
        let is_loader = dump.as_ref().map_or(false, |d| d.stage == "loader");
        if let Err(_) = self.load_script(path, is_loader) {
            return;
        }

        // Process the script, and generate reports.
        let is_parser = dump.as_ref().map_or(false, |d| d.stage == "parser");
        if let Err(_) = self.process_script(is_parser) {
            return;
        }

        let is_semantic = dump.as_ref().map_or(false, |d| d.stage == "semantic");
        if let Err(_) = self.semantic_analysis(is_semantic) {
            return;
        }

        let is_acyclic = dump.as_ref().map_or(false, |d| d.stage == "dag");
        if let Err(_) = self.acyclic_analysis(is_acyclic) {
            return;
        }

        let is_ir = dump.as_ref().map_or(false, |d| d.stage == "ir");
        if let Err(_) = self.ir_generation(is_ir) {
            return;
        }

        let _ = self
            .execute()
            .map(|_| {
                // Execution successful.
            })
            .map_err(|_| {
                // Error reports are generated within execute().
            });
    }

    /// Loads a script from the given path and stores it in the pipeline.
    /// If loading fails, a critical report is generated.
    /// # Arguments
    /// * `path` - The path to the script file to be loaded.
    /// * `dump` - A boolean indicating whether to dump the loaded script to a file.
    /// # Returns
    /// * `Ok(())` if the script is successfully loaded.
    /// * `Err(std::io::Error)` if there is an error loading the script
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js"));
    /// ```
    fn load_script(&mut self, path: &std::path::Path, dump: bool) -> Result<(), std::io::Error> {
        match script::Script::from_path(path) {
            Ok(script) => {
                self.script = Some(script);
                if dump {
                    if let Some(script) = &self.script {
                        std::fs::write("dump_loader.txt", &script.content).unwrap();
                    }
                }
                Ok(())
            }
            Err(e) => {
                let report = reports::Report::new(
                    reports::Level::Critical,
                    format!("Failed to load script: {}", e),
                    Some("Script Loader".into()),
                    None,
                    None,
                );
                report!(report);
                Err(e)
            }
        }
    }

    /// Processes the loaded script using the AST parser.
    /// If processing fails, an error report is generated.
    /// # Arguments
    /// * `dump` - A boolean indicating whether to dump the parser output to a file.
    /// # Returns
    /// * `Ok(())` if processing is successful.
    /// * `Err(())` if there is an error during processing.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js")).unwrap();
    /// pipeline.process_script(false).unwrap();
    /// ```
    fn process_script(&mut self, dump: bool) -> Result<(), ()> {
        if let Some(script) = &self.script {
            // Initialize the AST parser with the script content.
            parser::AstParser::new(&script)
                .map(|parser| {
                    self.parser = Some(parser);
                })
                .map_err(|_| {})?;

            // Expand includes in the script.
            parser::expander::expand_includes(
                &mut self.parser.as_mut().unwrap(),
                Rc::new(script.clone()),
            )?;

            if dump {
                if let Some(parser) = &self.parser {
                    std::fs::write("dump_parser.txt", format!("{:#?}", parser.root)).unwrap();
                }
            }

            Ok(())
        } else {
            let report = reports::Report::new(
                reports::Level::Error,
                "No script loaded to process.".into(),
                Some("Pipeline".into()),
                None,
                None,
            );
            report!(report);
            return Err(());
        }
    }

    /// Performs semantic analysis on the processed script.
    /// If analysis fails, an error report is generated.
    /// # Arguments
    /// * `dump` - A boolean indicating whether to dump the semantic analysis output to a file. This is currently unused since the semantic analyzer does not produce dumpable output.
    /// # Returns
    /// * `Ok(())` if analysis is successful.
    /// * `Err(())` if there is an error during analysis.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js")).unwrap();
    /// pipeline.process_script(false).unwrap();
    /// pipeline.semantic_analysis(false).unwrap();
    /// ```
    fn semantic_analysis(&mut self, _dump: bool) -> Result<(), ()> {
        if let Some(parser) = &self.parser {
            SemanticAnalyzer::new(parser.clone())
                .map(|analyzer| {
                    self.semantic = Some(analyzer);
                })
                .map_err(|_| {
                    let report = reports::Report::new(
                        reports::Level::Error,
                        "Semantic analysis failed.".into(),
                        Some("Pipeline".into()),
                        None,
                        None,
                    );
                    report!(report);
                })?;
            Ok(())
        } else {
            let report = reports::Report::new(
                reports::Level::Error,
                "No parser available for semantic analysis.".into(),
                Some("Pipeline".into()),
                None,
                None,
            );
            report!(report);
            return Err(());
        }
    }

    /// Performs acyclic analysis on the processed script.
    /// If analysis fails, an error report is generated.
    /// # Arguments
    /// * `dump` - A boolean indicating whether to dump the acyclic analysis output to a file.
    /// # Returns
    /// * `Ok(())` if analysis is successful.
    /// * `Err(())` if there is an error during analysis.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js")).unwrap();
    /// pipeline.process_script(false).unwrap();
    /// pipeline.acyclic_analysis(false).unwrap();
    /// ```
    fn acyclic_analysis(&mut self, dump: bool) -> Result<(), ()> {
        if let Some(parser) = &self.parser
            && let Some(semantic) = &self.semantic
        {
            DirectedAcyclicGraphAnalyzer::new(parser.clone(), Some(semantic.entry_point.clone()))
                .map(|analyzer| {
                    self.acyclic = Some(analyzer);
                })
                .map_err(|_| {
                    let report = reports::Report::new(
                        reports::Level::Error,
                        "Acyclic analysis failed.".into(),
                        Some("Pipeline".into()),
                        None,
                        None,
                    );
                    report!(report);
                })?;

            if dump {
                std::fs::write("dump_acyclic.txt", format!("{:#?}", self.acyclic)).unwrap();
            }

            Ok(())
        } else {
            let report = reports::Report::new(
                reports::Level::Error,
                "No parser or semantic analyzer available for acyclic analysis.".into(),
                Some("Pipeline".into()),
                None,
                None,
            );
            report!(report);
            return Err(());
        }
    }

    /// Generates intermediate representation (IR) from the processed script.
    /// If generation fails, an error report is generated.
    /// # Arguments
    /// * `dump` - A boolean indicating whether to dump the IR output to a file.
    /// # Returns
    /// * `Ok(())` if generation is successful.
    /// * `Err(())` if there is an error during generation.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js")).unwrap();
    /// pipeline.process_script(false).unwrap();
    /// pipeline.ir_generation(false).unwrap();
    /// ```
    fn ir_generation(&mut self, dump: bool) -> Result<(), ()> {
        if let Some(parser) = &self.parser {
            let ir = codegen::generate_program_from_ast(parser.root());
            if dump {
                std::fs::write("dump_ir.txt", format!("{:#?}", ir)).unwrap();
            }
            self.ir = Some(ir);
            Ok(())
        } else {
            let report = reports::Report::new(
                reports::Level::Error,
                "No parser available for IR generation.".into(),
                Some("Pipeline".into()),
                None,
                None,
            );
            report!(report);
            return Err(());
        }
    }

    /// Executes the generated bytecode.
    /// If execution fails, an error report is generated.
    /// # Examples
    /// ```
    /// let mut pipeline = Pipeline::new();
    /// pipeline.load_script(std::path::Path::new("example.js")).unwrap();
    /// pipeline.process_script(false).unwrap();
    /// pipeline.ir_generation(false).unwrap();
    /// pipeline.execution(false).unwrap();
    /// ```
    fn execute(&mut self) -> Result<(), ()> {
        if let Some(bytecode) = &self.ir {
            return runtime::execute(bytecode, self.script.as_ref().unwrap().path()).map_err(
                |op| {
                    let report = reports::Report::new(
                        reports::Level::Error,
                        format!("Runtime error during execution at op: {:?}", op),
                        Some("Runtime".into()),
                        None,
                        None,
                    );
                    report!(report);
                },
            );
        } else {
            let report = reports::Report::new(
                reports::Level::Error,
                "No bytecode available for execution.".into(),
                Some("Pipeline".into()),
                None,
                None,
            );
            report!(report);
            return Err(());
        }
    }
}
