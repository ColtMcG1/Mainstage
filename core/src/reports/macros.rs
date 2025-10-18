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