//! ./reports/accumulator.rs
//! 
//! A thread-safe singleton accumulator for managing reports.
//! This module provides a thread-safe singleton that accumulates and manages multiple reports.
//! It is designed to collect reports from various parts of the system and provide methods to retrieve, filter, and summarize them.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

use crate::reports::{Level, Report};
use std::sync::OnceLock;
use std::sync::{Arc, RwLock};

/// A thread-safe singleton that accumulates and manages multiple reports.
///
/// The `Accumulator` is designed to collect reports from various parts of the system
/// and provide methods to retrieve, filter, and summarize them.
///
/// # Examples
/// ```
/// let accumulator = Accumulator::get_instance();
/// accumulator.add_report(Report::new(Level::Info, "Test report"));
/// println!("{:?}", accumulator.reports());
/// ```
#[derive(Debug, Clone)]
pub struct Accumulator {
    reports: Arc<RwLock<Vec<Report<'static>>>>, // Use 'static lifetime for Report
}

/// Provides a default implementation for the Accumulator.
/// This implementation creates a new instance using the `new` method.
/// 
/// # Examples
/// ```
/// let accumulator = Accumulator::default();
/// accumulator.add_report(Report::new(Level::Info, "Test report"));
/// println!("{:?}", accumulator.reports());
/// ```
impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Accumulator {

    /// Creates a new `Accumulator` instance.
    /// This is a private constructor to enforce the singleton pattern.
    /// /// # Returns
    /// * A new `Accumulator` instance with an empty report list.
    fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Returns a reference to the singleton instance of the `Accumulator`.
    /// # Returns
    /// * A reference to the singleton `Accumulator` instance.
    pub fn get_instance() -> &'static Self {
        static INSTANCE: OnceLock<Accumulator> = OnceLock::new();
        INSTANCE.get_or_init(|| Accumulator::new())
    }

    /// Adds a single report to the accumulator.
    /// # Arguments
    /// * `report` - The `Report` instance to be added.
    /// # Returns
    /// * A reference to the singleton `Accumulator` instance.
    pub fn add_report(&self, report: Report<'static>) {
        let mut reports = self.reports.write().unwrap(); // Write lock for modifying reports
        reports.push(report);
    }

    /// Adds multiple reports to the accumulator.
    /// # Arguments
    /// * `other` - A vector of `Report` instances to be added.
    /// # Returns
    /// * A reference to the singleton `Accumulator` instance.
    pub fn extend(&self, other: Vec<Report<'static>>) {
        let mut reports = self.reports.write().unwrap();
        reports.extend(other);
    }

    /// Returns a reference to all reports.
    /// # Returns
    /// * A vector of all `Report` instances in the accumulator.
    pub fn get_reports<'a>(&'a self) -> Vec<Report<'a>> {
        let reports = self.reports.read().unwrap(); // Read lock for accessing reports
        reports.clone() // Return a cloned copy of the reports
    }

    /// Clears all reports from the accumulator.
    pub fn clear(&self) {
        let mut reports = self.reports.write().unwrap();
        reports.clear();
    }

    /// Checks if the accumulator is empty.
    /// # Returns
    /// * `true` if there are no reports, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        let reports = self.reports.read().unwrap();
        reports.is_empty()
    }

    /// Returns the number of reports in the accumulator.
    /// # Returns
    /// * The count of reports as `usize`.
    pub fn len(&self) -> usize {
        let reports = self.reports.read().unwrap();
        reports.len()
    }

    /// Removes and returns all reports from the accumulator.
    /// # Returns
    /// * A vector of all `Report` instances in the accumulator.
    pub fn drain(&self) -> Vec<Report<'static>> {
        let mut reports = self.reports.write().unwrap();
        std::mem::take(&mut *reports)
    }

    /// Filters reports based on a predicate function.
    /// # Arguments
    /// * `predicate` - A closure that takes a reference to a `Report` and returns a `bool`.
    /// # Returns
    /// * A vector of `Report` instances that satisfy the predicate.
    pub fn filter_reports<'a, F>(&'a self, predicate: F) -> Vec<Report<'a>>
    where
        F: Fn(&Report) -> bool,
    {
        let reports = self.reports.read().unwrap();
        reports
            .iter()
            .filter(|&report| predicate(report))
            .cloned()
            .collect()
    }

    /// Retrieves all reports of a specific `ReportLevel`.
    /// # Arguments
    /// * `level` - The `ReportLevel` to filter reports by.
    /// # Returns
    /// * A vector of `Report` instances with the specified level.
    pub fn reports_by_level<'a>(&'a self, level: Level) -> Vec<Report<'a>> {
        self.get_reports()
            .into_iter()
            .filter(|report| report.level == level)
            .collect()
    }

    /// Summarizes the number of reports by `ReportLevel`.
    /// # Returns
    /// * A vector of tuples containing the `ReportLevel` and its corresponding count.
    pub fn summarize_by_level(&self) -> Vec<(Level, usize)> {
        let mut summary = std::collections::HashMap::new();

        for report in self.get_reports() {
            *summary.entry(report.level).or_insert(0) += 1;
        }

        summary.into_iter().collect()
    }

    /// Checks if there are any reports of a specific `ReportLevel`.
    /// # Arguments
    /// * `level` - The `ReportLevel` to check for.
    /// # Returns
    /// * `true` if there are reports of the specified level, `false` otherwise.
    pub fn has_level(&self, level: Level) -> bool {
        let reports = self.reports.read().unwrap();
        reports.iter().any(|report| report.level == level)
    }

    /// Converts the summary of reports by level into a formatted string.
    /// # Returns
    /// * A formatted string summarizing the reports by level.
    pub fn to_string(&self) -> String {
        let mut summary = String::new();
        for (level, count) in self.summarize_by_level() {
            summary.push_str(&format!("{:?}: {}\n", level, count));
        }
        summary
    }
}

/// Allows iteration over the reports in the accumulator.
/// Implements the `IntoIterator` trait for the `Accumulator`.
impl<'a> IntoIterator for Accumulator {
    type Item = Report<'static>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let reports = self.reports.read().unwrap();
        reports.clone().into_iter()
    }
}
