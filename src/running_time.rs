//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! ## Type
//!
//! * RunningTime
//!     * RunningTime accumulates statistics on a stream of event times.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use std::time::Instant;
//!    use rustics::Rustics;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::running_time::RunningTime;
//!
//!    // Create an instance to record query latencies.  This is for a
//!    // time statistic, so we need a timer.  Use an adapter for the
//!    // Rust standard Duration timer.
//!
//!    // The default for printing output is stdout, which we'll assume
//!    // is fine, so None will work for the printer option.  See the
//!    // Printer trait in lib.rs for information on writing a custom
//!    // Printer.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    let mut query_latency =
//!        RunningTime::new("Query Latency", timer, None);
//!
//!    // By way of example, we assume that the queries are single-
//!    // threaded, so we can use the record_time() method to query the
//!    // timer and restart it.
//!    //
//!    // So record one time sample for the single-threaded case.  The
//!    // timer started running when we created the Duration timer.
//!
//!    query_latency.record_event();
//!
//!    // For the multithreaded case, you can use DurationTimer manually.
//!
//!    let mut local_timer = DurationTimer::new();
//!
//!    // Do our query.
//!    // ...
//!
//!    // You can use the finish() method if this RunningTime instance is
//!    // shared.
//!
//!    query_latency.record_time(local_timer.finish() as i64);
//!
//!    // If you want to use your own timer, you'll need to implement the
//!    // Timer trait to initialize the RunningTime instance, but you can
//!    // use it directly to get data. Let's use Duration timer directly
//!    // as an example.  Make a new instance for this example.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    let mut query_latency =
//!        RunningTime::new("Custom Timer", timer.clone(), None);
//!
//!    // Start the Duration timer.
//!
//!    let start = Instant::now();
//!
//!    // Do our query.
//!
//!    // Now get the elapsed time.  DurationTimer works in nanoseconds,
//!    // so use as_nanos() to get the tick count.
//!
//!    let time_spent = start.elapsed().as_nanos();
//!    assert!(timer.borrow().hz() == 1_000_000_000);
//!
//!    query_latency.record_time(time_spent as i64);
//!
//!    // Print our statistics.  This example has only one event recorded.
//!
//!    query_latency.print();
//!
//!    assert!(query_latency.count() == 1);
//!    assert!(query_latency.mean() == time_spent as f64);
//!    assert!(query_latency.standard_deviation() == 0.0);
//!```

use std::any::Any;

use super::Rustics;
use super::Units;
use super::Histogram;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::HistogramBox;
use super::parse_print_opts;
use super::stdout_printer;
use super::TimerBox;
use super::timer_box_hz;
use super::printable::Printable;
use super::running_integer::RunningInteger;
use super::running_integer::RunningExport;

/// A RunningTime instance accumulates statistics on a stream
/// of integer data samples representing time intervals.
/// Underneath, it uses a RunningTime instance, but most
/// output is printed as time periods.
///
/// See the module comments for a sample program.

#[derive(Clone)]
pub struct RunningTime {
    running_integer:    Box<RunningInteger>,
    timer:              TimerBox,
    hz:                 i64,

    printer:            PrinterBox,
}

impl RunningTime {
    /// Creates a new RunningTime instance

    pub fn new(name_in: &str, timer: TimerBox, printer: PrinterOption) -> RunningTime {
        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::RunningTime:  The timer hz value is too large.");
        }

        let hz              = hz as i64;
        let running_integer = Box::new(RunningInteger::new(name_in, printer.clone()));

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RunningTime { printer, running_integer, timer, hz }
    }

    pub fn new_opts(name: &str, timer: TimerBox, print_opts: &PrintOption) -> RunningTime {
        let hz = timer_box_hz(&timer);

        let (printer, _title, _units) = parse_print_opts(print_opts, name);

        if hz > i64::MAX as u128 {
            panic!("Rustics::RunningTime:  The timer hz value is too large.");
        }

        let hz              = hz as i64;
        let running_integer = Box::new(RunningInteger::new_opts(name, print_opts));

        RunningTime { printer, running_integer, timer, hz }
    }

    /// Creates a RunningTime instance from a RunningInteger.  This function
    /// is used internally to support the Hier code.

    pub fn from_integer(timer: TimerBox, print_opts: &PrintOption, mut running: RunningInteger)
            -> RunningTime {
        let (printer, title, _units) = parse_print_opts(print_opts, &running.name());

        running.set_title(&title);
        running.set_units(Units::empty());

        let hz              = timer_box_hz(&timer) as i64;
        let running_integer = Box::new(running);

        RunningTime { running_integer, timer, hz, printer }
    }

    pub fn hz(&self) -> i64 {
        self.hz
    }

    // This function is used by RunningTime as it is simply
    // a wrapper for a RunningInteger.

    /// Exports the statistics for this instance.

    pub fn export(&self) -> RunningExport {
        self.running_integer.export_all()
    }
}

impl Rustics for RunningTime {
    fn record_i64(&mut self, _sample: i64) {
        panic!("Rustics::RunningTime:  i64 events are not permitted.");
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::RunningTime:  f64 events are not permitted.");
    }

    fn record_event(&mut self) {
        let _ = self.record_event_report();
    }

    fn record_event_report(&mut self) -> i64 {
        let mut timer    = (*self.timer).borrow_mut();
        let     interval = timer.finish();  // read and restart the timer

        self.running_integer.record_i64(interval);
        interval
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.running_integer.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
        let interval = timer.finish();

        self.running_integer.record_i64(interval);
    }

    fn name(&self) -> String {
        self.running_integer.name()
    }

    fn title(&self) -> String {
        self.running_integer.title()
    }

    fn class(&self) -> &str {
        "time"
    }

    fn count(&self) ->u64 {
        self.running_integer.count()
    }

    fn log_mode(&self) -> isize {
        self.running_integer.log_mode()
    }

    fn mean(&self) ->f64 {
        self.running_integer.mean()
    }

    fn standard_deviation(&self) ->f64 {
        self.running_integer.standard_deviation()
    }

    fn variance(&self) ->f64 {
        self.running_integer.variance()
    }

    fn skewness(&self) ->f64 {
        self.running_integer.skewness()
    }

    fn kurtosis(&self) ->f64 {
        self.running_integer.kurtosis()
    }

    fn int_extremes(&self) -> bool {
        self.running_integer.int_extremes()
    }

    fn min_i64(&self) -> i64 {
        self.running_integer.min_i64()
    }

    fn min_f64(&self) -> f64 {
        self.running_integer.min_f64()
    }

    fn max_i64(&self) -> i64 {
        self.running_integer.max_i64()
    }

    fn max_f64(&self) -> f64 {
        self.running_integer.max_f64()
    }

    fn precompute(&mut self) {
        self.running_integer.precompute()
    }

    fn clear(&mut self) {
        self.running_integer.clear()
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
                &self.running_integer.title()
            };

        let n        = self.count();
        let min      = self.min_i64();
        let max      = self.max_i64();
        let log_mode = self.running_integer.log_mode() as i64;
        let mean     = self.mean();
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();
        let units    = Units::empty();

        let printable =
            Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis, units };

        let printer  = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);
        self.running_integer.print_histogram(printer);
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.running_integer.set_title(title);
    }

    fn set_id(&mut self, id: usize) {
        self.running_integer.set_id(id)
    }

    fn id(&self) -> usize {
        self.running_integer.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        self.running_integer.equals(other)
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histogram(&self) -> HistogramBox {
        self.running_integer.histogram()
    }

    fn export_stats(&self) -> (Printable, HistogramBox) {
        self.running_integer.export_stats()
    }
}

impl Histogram for RunningTime {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.running_integer.print_histogram(printer)
    }
}
