//! ./lib.rs
//!
//! Core library for the Mainstage project.
//! This library provides the core functionality for the Mainstage project, including report management and script handling.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

use crate::reports::*;
use crate::scripts::*;
use crate::semantic::*;

#[macro_use]
pub mod reports;

pub mod parser;
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
        // TODO: Further processing stages would go here, each potentially generating reports.
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
    /// * `dump` - A boolean indicating whether to dump the semantic analysis output to a file.
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
    fn semantic_analysis(&mut self, dump: bool) -> Result<(), ()> {
        if let Some(parser) = &self.parser {
            SemanticAnalyzer::new(parser.clone())
                .map(|analyzer| {
                    if dump {
                        std::fs::write(
                            "dump_semantic.txt",
                            format!("{:#?}", analyzer.symbol_table),
                        )
                        .unwrap();
                    }
                })
                .map_err(|_| {})?;
        }
        Ok(())
    }
}
