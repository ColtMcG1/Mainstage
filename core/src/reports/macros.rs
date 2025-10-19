//! ./reports/macros.rs
//! 
//! Macros for reporting issues within the system.
//! This module provides macros to facilitate the creation and accumulation of reports.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

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