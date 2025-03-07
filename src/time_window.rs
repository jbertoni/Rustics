//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkeley 3-clause,
//  and MIT licenses.
//

//! ## Type
//!
//! * TimerWindow
//!     * TimerWindow maintains a set consisting of the last n time
//!       intervals recorded into it.
//!
//!     * This type uses IntegerWindow internally to record the time
//!       samples.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use rustics::Rustics;
//!    use rustics::timer_mut;
//!    use rustics::time_window::TimeWindow;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::running_time::RunningTime;
//!
//!    // Create an instance to record packet latencies.
//!    //
//!    // Use a DurationTimer to measure time.  DurationTimer is a wrapper
//!    // for the standard Rust Duration type.  This example is for a single-
//!    // threaded instances.  See ArcSet for an example of multi-threading
//!    // for time statistics.
//!
//!    let mut timer = DurationTimer::new_box();
//!
//!    // Retain 1000 samples in the window.
//!
//!    let window_size = 1000;
//!
//!    // Assume that the default print options are fine, so None works
//!    // for that parameter.  See the RunningInteger comments for an
//!    // example of how to set print options.
//!
//!    let mut packet_latency =
//!        TimeWindow::new("Packet Latency", window_size, timer.clone(), &None);
//!
//!    // Record some hypothetical packet latencies.  The clock started
//!    // running when we created the timer.  Use the start() method to
//!    // set a new start time.
//!
//!    timer_mut!(timer).start();
//!
//!    for i in 1..=window_size {
//!       // Do work...
//!
//!       packet_latency.record_event();
//!       assert!(packet_latency.count() == i as u64);
//!    }
//!
//!    // Print our statistics.
//!
//!    packet_latency.print();
//!
//!    // We should have seen "window_size" events.
//!
//!    assert!(packet_latency.count() == window_size as u64);
//!
//!    // Record more samples, and check that the count is now constant.
//!
//!    for i in 1..=window_size / 2 {
//!       packet_latency.record_event();
//!       assert!(packet_latency.count() == window_size as u64);
//!    }
//!
//!```

use std::any::Any;

use super::Rustics;
use super::ExportStats;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::TimerBox;
use super::Histogram;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use super::timer_box_hz;
use super::parse_print_opts;
use super::printer_mut;
use super::timer_mut;
use super::integer_window::IntegerWindow;

/// TimeWindow implements a Rustics type that retains a
/// window of the last n samples of a stream of data samples.
/// It also provides a histogram of all data samples recorded.
///
/// See the module comments for a sample program.

#[derive(Clone)]
pub struct TimeWindow {
    integer_window:     Box<IntegerWindow>,
    timer:              TimerBox,
    hz:                 i64,
    printer:            PrinterBox,
    //units:              Units,
}

impl TimeWindow {
    /// Makes a new TimeWindow instance.

    pub fn new(name: &str, window_size: usize, timer: TimerBox, print_opts: &PrintOption)
            -> TimeWindow {
        let (printer, _title, _units, _histo_opts) = parse_print_opts(print_opts, name);

        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::TimeWindow:  The timer hz value is too large.");
        }

        let hz             = hz as i64;
        let integer_window = IntegerWindow::new(name, window_size, print_opts);
        let integer_window = Box::new(integer_window);

        TimeWindow { printer, integer_window, timer, hz }
   }

    /// Returns the frequency of the Timer instance being used
    /// by this instance.

    pub fn hz(&self) -> i64 {
        self.hz
    }
}

impl Rustics for TimeWindow {
    fn record_i64(&mut self, _sample: i64) {
        panic!("Rustics::TimeWindow:  i64 events are not permitted.");
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::TimeWindow:  f64 events are not permitted.");
    }

    /// Records a time value obtained from the timer instance used to
    /// to create this TimeWindow instance.  Calling finish() on the
    /// timer automatically starts the new interval.  This method
    /// only works for single-threaded statistics gathering.

    fn record_event(&mut self) {
        let _ = self.record_event_report();
    }

    fn record_event_report(&mut self) -> i64 {
        let interval = timer_mut!(*self.timer).finish();

        self.integer_window.record_i64(interval);
        interval
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.integer_window.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let timer    = timer_mut!(*timer);
        let interval = timer.finish();

        self.integer_window.record_i64(interval);
    }

    fn name(&self) -> String {
        self.integer_window.name()
    }

    fn title(&self) -> String {
        self.integer_window.title()
    }

    fn class(&self) -> &str {
        "time"
    }

    fn count(&self) ->u64 {
        self.integer_window.count()
    }

    fn log_mode(&self) -> isize {
        self.integer_window.log_mode()
    }

    fn mean(&self) ->f64 {
        self.integer_window.mean()
    }

    fn standard_deviation(&self) ->f64 {
        self.integer_window.standard_deviation()
    }

    fn variance(&self) ->f64 {
        self.integer_window.variance()
    }

    fn skewness(&self) ->f64 {
        self.integer_window.skewness()
    }

    fn kurtosis(&self) ->f64 {
        self.integer_window.kurtosis()
    }

    fn int_extremes(&self) -> bool {
        self.integer_window.int_extremes()
    }

    fn float_extremes(&self) -> bool {
        self.integer_window.float_extremes()
    }

    fn min_i64(&self) -> i64 {
        self.integer_window.min_i64()
    }

    fn min_f64(&self) -> f64 {
        self.integer_window.min_f64()
    }

    fn max_i64(&self) -> i64 {
        self.integer_window.max_i64()
    }

    fn max_f64(&self) -> f64 {
        self.integer_window.max_f64()
    }

    fn precompute(&mut self) {
        self.integer_window.precompute()
    }

    fn clear(&mut self) {
        self.integer_window.clear()
    }

    // Functions for printing

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer_box =
            if let Some(printer) = printer {
                printer
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.integer_window.title()
            };

        let printable = self.integer_window.get_printable();
        let printer   = printer_mut!(printer_box);

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);
        self.integer_window.print_histogram(printer);
        printer.print("");
    }

    fn set_title(&mut self, title: &str) {
        self.integer_window.set_title(title)
    }

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        self.integer_window.log_histogram()
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }

    // For internal use only.

    fn set_id(&mut self, id: usize) {
        self.integer_window.set_id(id)
    }

    fn id(&self) -> usize {
        self.integer_window.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<TimeWindow>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn export_stats(&self) -> ExportStats {
        self.integer_window.export_stats()
    }
}

impl Histogram for TimeWindow {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.integer_window.print_histogram(printer);
    }

    fn clear_histogram(&mut self) {
        self.integer_window.clear_histogram();
    }

    fn to_log_histogram(&self) -> Option<LogHistogramBox> {
        self.integer_window.to_log_histogram()
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        self.integer_window.to_float_histogram()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use crate::PrintOpts;
    use crate::counter::Counter;
    use crate::stdout_printer;
    use crate::timer;
    use crate::timer_box;
    use crate::time::ClockTimer;
    use crate::time::DurationTimer;
    use crate::running_time::tests::LargeTimer;
    use crate::tests::compute_sum;
    use crate::tests::continuing_box;
    use crate::tests::check_printer_box;
    use crate::time::tests::TestSimpleClock;

    fn simple_test() {
        let     size  = 200;
        let     timer = continuing_box();
        let mut stat  = TimeWindow::new("Test Time Window", size, timer, &None);

        let     timer = continuing_box();
        let     timer = timer!(timer);
        let     hz    = timer.hz();

        assert!(stat.hz() == hz as i64);

        for i in 1..=size {
            stat.record_time(i as i64);
            assert!(stat.count() == i as u64)
        }

        assert!(stat.log_mode() == 8);

        // precompute() should be harmless.

        stat.precompute();

        let count = size as f64;
        let sum   = (count * (count + 1.0)) / 2.0;
        let mean  = sum / count;

        assert!(stat.count() == size as u64);
        assert!(stat.mean () == mean       );

        let export = stat.export_stats();

        assert!(export.printable.n    == size as u64);
        assert!(export.printable.mean == mean       );
    }

    fn test_equality() {
        let size   = 200;
        let timer  = continuing_box();
        let stat_1 = TimeWindow::new("Equality Test 1", size, timer, &None);

        let timer  = continuing_box();
        let stat_2 = TimeWindow::new("Equality Test 2", size, timer, &None);
        let stat_3 = Counter::new("Equality Test 2", &None);

        assert!( stat_1.equals(&stat_1));
        assert!(!stat_1.equals(&stat_2));
        assert!(!stat_1.equals(&stat_3));

        let generic = stat_1.generic();
        let recast  = generic.downcast_ref::<TimeWindow>().unwrap();

        assert!(stat_1.equals(recast));
    }

    #[test]
    #[should_panic]
    fn test_record_i64() {
        let     size  = 200;
        let     timer = continuing_box();
        let mut stat  = TimeWindow::new("Test Time Window", size, timer, &None);
        let     _     = stat.record_i64(1 as i64);
    }

    #[test]
    #[should_panic]
    fn test_max_f64() {
        let size  = 200;
        let timer = continuing_box();
        let stat  = TimeWindow::new("Test Time Window", size, timer, &None);
        let _     = stat.max_f64();
    }

    #[test]
    #[should_panic]
    fn test_min_f64() {
        let size  = 200;
        let timer = continuing_box();
        let stat  = TimeWindow::new("Test Time Window", size, timer, &None);
        let _     = stat.min_f64();
    }

    #[test]
    #[should_panic]
    fn test_record_f64() {
        let     size  = 200;
        let     timer = continuing_box();
        let mut stat  = TimeWindow::new("Test Time Window", size, timer, &None);

        let _ = stat.record_f64(1.0);
    }

    #[test]
    #[should_panic]
    fn test_float_histogram() {
        let size  = 200;
        let timer = continuing_box();
        let stat  = TimeWindow::new("Test Time Window", size, timer, &None);

        let _ = stat.float_histogram().unwrap();
    }

    fn test_histogram() {
        let     printer = stdout_printer();
        let     printer = printer_mut!(printer);
        let     size    = 200;
        let     timer   = continuing_box();
        let mut stat    = TimeWindow::new("Test Time Window", size, timer, &None);

        for i in 1..=size {
            stat.record_time(i as i64);
            assert!(stat.count() == i as u64)
        }

        {
            let histogram = stat.to_log_histogram().unwrap();
            let histogram = histogram.borrow();
            let sum       = compute_sum(&histogram);

            assert!(sum == size as i64);
        }

        stat.print_histogram(printer);
        stat.clear_histogram();

        {
            let histogram = stat.to_log_histogram().unwrap();
            let histogram = histogram.borrow();
            let sum       = compute_sum(&histogram);

            assert!(sum == 0);
        }
    }

    #[test]
    #[should_panic]
    fn test_to_float_histogram() {
        let size  = 200;
        let timer = continuing_box();
        let stat  = TimeWindow::new("Test Time Window", size, timer, &None);
        let _     = stat.to_float_histogram().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_large_clock() {
        let size  = 200;
        let timer = timer_box!(LargeTimer { });
        let _     = TimeWindow::new("Test Time Window", size, timer, &None);
    }

    fn test_print_output() {
        let expected =
            [
                "Test Statistics",
                "    Count               1,000 ",
                "    Minimum             1.000 microsecond",
                "    Maximum             1.000 millisecond",
                "    Log Mode               20 ",
                "    Mode Value        786.432 microseconds",
                "    Mean              500.500 microseconds",
                "    Std Dev           288.819 microseconds",
                "    Variance         +8.34166 e+10 ",
                "    Skewness         +0.00000 e+0  ",
                "    Kurtosis         -1.20000 e+0  ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 0",
                "    4:                 0                 0                 0                 0",
                "    8:                 0                 0                 1                 1",
                "   12:                 2                 4                 8                16",
                "   16:                33                66               131               262",
                "   20:               476                 0                 0                 0",
                ""
            ];

        let     timer      = continuing_box();
        let     printer    = Some(check_printer_box(&expected, true, false));
        let     title      = None;
        let     units      = None;
        let     histo_opts = None;
        let     print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let     name       = "Test Statistics";
        let     samples    = 1000;
        let mut stats      = TimeWindow::new(&name, samples, timer, &print_opts);

        for _i in 1..=samples {
            stats.record_event();
        }

        stats.print();
    }

    fn test_timer_boxes() {
        let     current      = 1000;
        let     increment    = 2;
        let     window_size  = 1000;
        let     tests        = window_size / 10;
        let     simple_clock = TestSimpleClock { current, increment };
        let     simple_clock = timer_box!(simple_clock);
        let mut clock_timer  = ClockTimer::new_box(simple_clock);
        let mut time_window  = TimeWindow::new("Simple", window_size, clock_timer.clone(), &None);

        for _i in 0..tests {
            time_window.record_event();
            time_window.record_interval(&mut clock_timer);
        }

        assert!(time_window.count() == tests as u64 * 2);

        let mut duration     = DurationTimer::new_box();
        let mut time_window  = TimeWindow::new("Duration", window_size, duration.clone(), &None);

        for _i in 0..tests {
            time_window.record_event();
            time_window.record_interval(&mut duration);
        }

        assert!(time_window.count() == tests as u64 * 2);
    }

    #[test]
    fn run_tests() {
        simple_test      ();
        test_equality    ();
        test_histogram   ();
        test_print_output();
        test_timer_boxes ();
    }
}
