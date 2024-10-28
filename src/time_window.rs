//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! ## Type
//!
//! * TimerWindow
//!     * TimerWindow maintains a set consisting of the last n samples
//!       recorded into it.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use rustics::Rustics;
//!    use rustics::time_window::TimeWindow;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::running_time::RunningTime;
//!
//!    // Create  an instance to record packet latencies.  The default
//!    // for printing output is stdout, which we'll assume is fine for
//!    // this example, so None works for the printer.
//!    //
//!    // Assume that retaining 1000 samples is fine, and use the
//!    // DurationTimer to measure time.  DurationTimer is a wrapper for
//!    // the standard Rust Duration type.  This example is for a single-
//!    // threaded statistics instances.  See ArcSet for an example of
//!    // multi-threading for time statistics.
//!
//!    // Retain 1000 samples.
//!
//!    let window_size = 1000;
//!    let mut timer = DurationTimer::new_box();
//!
//!    let mut packet_latency =
//!        TimeWindow::new("Packet Latency", window_size, timer.clone(), None);
//!
//!    // Record some hypothetical packet latencies.  The clock started
//!    // running when we created the timer.  Use the start() method to
//!    // set a new start time.
//!
//!    timer.borrow_mut().start();
//!
//!    for i in 1..window_size + 1 {
//!       // Do work...
//!
//!       packet_latency.record_event();
//!       assert!(packet_latency.count() == i as u64);
//!    }
//!
//!    // Print our statistics.  This example has only one event recorded.
//!
//!    packet_latency.print();
//!
//!    // We should have seen "window_size" events.
//!
//!    assert!(packet_latency.count() == window_size as u64);
//!
//!    for i in 1..window_size / 2 + 1 {
//!       packet_latency.record_event();
//!       assert!(packet_latency.count() == window_size as u64);
//!    }
//!
//!```

use std::any::Any;

use super::Rustics;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::TimerBox;
use super::Histogram;
use super::HistogramBox;
use super::timer_box_hz;
use super::parse_print_opts;
use super::integer_window::IntegerWindow;
use crate::printable::Printable;
use super::stdout_printer;

/// TimeWindow implements a statistics type that retains a
//  window of the last n samples of astream of data samples
/// and a histogram of all data samples recorded.
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
    /// Make a new TimeWindow instance.

    pub fn new(name: &str, window_size: usize, timer:  TimerBox, printer: PrinterOption)
            -> TimeWindow {
        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::TimeWindow:  The timer hz value is too large.");
        }

        let hz             = hz as i64;
        let integer_window = IntegerWindow::new(name, window_size, printer.clone());
        let integer_window = Box::new(integer_window);

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        TimeWindow { printer, integer_window, timer, hz }
    }

    pub fn new_opts(name: &str, window_size: usize, timer: TimerBox, print_opts: &PrintOption) -> TimeWindow {
        let (printer, _title, _units) = parse_print_opts(print_opts, name);

        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::TimeWindow:  The timer hz value is too large.");
        }

        let hz             = hz as i64;
        let integer_window = IntegerWindow::new_opts(name, window_size, print_opts);
        let integer_window = Box::new(integer_window);

        TimeWindow { printer, integer_window, timer, hz }
   }

    /// Returns the hertz rating of the Timer instance being used
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
        let interval = (*self.timer).borrow_mut().finish();

        self.integer_window.record_i64(interval);
        interval
    }

    /// Records a time sample measured in ticks.

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.integer_window.record_i64(sample);
    }

    /// Records an interval by reading the timer provided.

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
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

    /// Returns the most common pseudo-log value from the data.

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
        let printer   = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);
        self.integer_window.print_histogram(printer);
    }

    // For internal use only.

    fn set_title(&mut self, title: &str) {
        self.integer_window.set_title(title)
    }

    fn set_id(&mut self, id: usize) {
        self.integer_window.set_id(id)
    }

    fn id(&self) -> usize {
        self.integer_window.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        self.integer_window.equals(other)
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histogram(&self) -> HistogramBox {
        self.integer_window.histogram()
    }

    fn export_stats(&self) -> (Printable, HistogramBox) {
        self.integer_window.export_stats()
    }
}

impl Histogram for TimeWindow {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.histogram().borrow().print(printer)
    }

    fn clear_histogram(&mut self) {
        self.histogram().borrow_mut().clear();
    }

    fn to_log_histogram(&self) -> Option<HistogramBox> {
        Some(self.histogram())
    }
}
