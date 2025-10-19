//! ./reports/macros.rs
//! 
//! Macros for reporting issues within the system.
//! This module provides macros to facilitate the creation and accumulation of reports.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

/// Macro to create and add a report to the accumulator.
/// # Arguments:
/// - level: The severity level of the report.
/// - message: The message describing the report.
/// - issuer: (Optional) The issuer of the report.
/// - span: (Optional) The span associated with the report.
/// - location: (Optional) The location associated with the report.
/// # Examples
/// ```rust
/// use mainstage::reports::report;
/// report!(
///    Level::Error,
///    "An error occurred",
///    Some("mainstage.module.function".to_string()),
///    None,
///    None
/// );
/// ```
#[macro_export]
macro_rules! report {
    // Full report with all fields
    ($level:expr, $message:expr, $issuer:expr, $span:expr, $location:expr) => {
        $crate::reports::accumulator::Accumulator::get_instance().add_report(
            $crate::reports::Report::new(
                $level,
                $message,
                $issuer,
                $span,
                $location,
            ),
        );
    };

    // Report without span and location
    ($level:expr, $message:expr, $issuer:expr) => {
        $crate::reports::accumulator::Accumulator::get_instance().add_report(
            $crate::reports::Report::new(
                $level,
                $message,
                $issuer,
                None,
                None,
            ),
        );
    };

    ($report:expr) => {
        $crate::reports::accumulator::Accumulator::get_instance().add_report($report);
    };
}