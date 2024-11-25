//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//! ## Type
//!
//! * RunningTime
//!     * RunningTime accumulates statistics on a stream of time intervals.
//!
//!     * Internally, it uses RunningInteger instances to collect statistics.
//!       See that code for more information
//!
//!     * The main.rs program contains a simple example of how to use this
//!       type.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use std::time::Instant;
//!    use rustics::Rustics;
//!    use rustics::timer;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::running_time::RunningTime;
//!
//!    // Create an instance to record query latencies.  This is for a
//!    // time statistic, so we need a timer.  Use an adapter for the
//!    // Rust standard Duration timer.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    // Accept the default print options. See the RunningInteger
//!    // comments for an example of how to set print options.
//!
//!    let mut query_latency =
//!        RunningTime::new("Query Latency", timer, &None);
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
//!    // record_interval() works, as well.  Create (and start) a timer,
//!    // then record the interval.
//!
//!    let mut timer_box = DurationTimer::new_box();
//!
//!    // do_work();
//!
//!    query_latency.record_interval(&mut timer_box);
//!
//!    // If you want to use your own timer, you'll need to implement the
//!    // Timer trait to initialize the RunningTime instance, but you can
//!    // use it directly to get data. Let's use Duration timer directly
//!    // as an example.  Make a new instance for this example.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    let mut query_latency =
//!        RunningTime::new("Custom Timer", timer.clone(), &None);
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
//!    assert!(timer!(timer).hz() == 1_000_000_000);
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
use super::ExportStats;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use super::parse_print_opts;
use super::TimerBox;
use super::printer_mut;
use super::timer_mut;
use super::timer_box_hz;
use super::running_integer::RunningInteger;
use super::merge::Export;

/// A RunningTime instance accumulates statistics on a stream
/// of integer data samples representing time intervals.
/// Underneath, it uses a RunningInteger instance, but most
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
    /// Creates a new RunningTime instance.

    pub fn new(name: &str, timer: TimerBox, print_opts: &PrintOption) -> RunningTime {
        let hz = timer_box_hz(&timer);

        let (printer, _title, _units, _histo_opts) = parse_print_opts(print_opts, name);

        if hz > i64::MAX as u128 {
            panic!("Rustics::RunningTime:  The timer hz value is too large.");
        }

        let hz              = hz as i64;
        let running_integer = Box::new(RunningInteger::new(name, print_opts));

        RunningTime { printer, running_integer, timer, hz }
    }

    /// Creates a RunningTime instance from a RunningInteger.  This function
    /// is used internally to support the Hier code.

    pub fn from_integer(timer: TimerBox, print_opts: &PrintOption, mut running: RunningInteger)
            -> RunningTime {
        let (printer, title, _units, _histo_opts) = parse_print_opts(print_opts, &running.name());

        running.set_title(&title);
        running.set_units(Units::empty());

        let hz              = timer_box_hz(&timer) as i64;
        let running_integer = Box::new(running);

        RunningTime { running_integer, timer, hz, printer }
    }

    // This function is used by RunningTime as it is simply
    // a wrapper for a RunningInteger.

    /// Exports the statistics for this instance.

    pub fn export(&self) -> Export {
        self.running_integer.export_data()
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
        let timer    = timer_mut!(*self.timer);
        let interval = timer.finish();  // read and restart the timer

        self.running_integer.record_i64(interval);
        interval
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.running_integer.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let timer    = timer_mut!(*timer);
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

    fn float_extremes(&self) -> bool {
        self.running_integer.float_extremes()
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

        let printable = self.running_integer.get_printable();
        let printer   = printer_mut!(printer_box);

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);
        self.running_integer.print_histogram(printer);
        printer.print("");
    }

    fn set_title(&mut self, title: &str) {
        self.running_integer.set_title(title);
    }

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        self.running_integer.log_histogram()
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        self.running_integer.float_histogram()
    }

    // For internal use only.

    fn set_id(&mut self, id: usize) {
        self.running_integer.set_id(id)
    }

    fn id(&self) -> usize {
        self.running_integer.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<RunningTime>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn export_stats(&self) -> ExportStats {
        self.running_integer.export_stats()
    }
}

impl Histogram for RunningTime {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.running_integer.print_histogram(printer)
    }

    fn clear_histogram(&mut self) {
        self.running_integer.clear_histogram();
    }

    fn to_log_histogram(&self) -> Option<LogHistogramBox> {
        self.running_integer.log_histogram()
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        self.running_integer.float_histogram()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::PrintOpts;
    use crate::stdout_printer;
    use crate::timer_box;
    use crate::tests::continuing_box;
    use crate::tests::compute_sum;
    use crate::tests::check_printer_box;
    use crate::hier::HierMember;
    use crate::counter::Counter;
    use crate::time::Timer;
    use std::rc::Rc;
    use std::cell::RefCell;

    fn simple_test() {
        println!("RunningTime::simple_test:  starting");

        let     timer        = continuing_box();
        let mut stat         = RunningTime::new("Query Latency", timer, &None);
        let     printer      = stdout_printer();
        let     printer      = printer_mut!(printer);
        let     sample_count = 200;

        for i in 1..=sample_count {
            stat.record_time(i);
        }

        assert!( stat.log_mode      () == 8);
        assert!( stat.int_extremes  ()     );
        assert!(!stat.float_extremes()     );

        assert!(stat.max_i64() == sample_count);
        assert!(stat.min_i64() == 1           );

        // precompute() should be a harmess nopl

        stat.precompute();

        assert!( stat.log_mode      () == 8);
        assert!( stat.int_extremes  ()     );
        assert!(!stat.float_extremes()     );

        let histogram = stat.to_histogram();

        histogram.print_histogram(printer);

        {
            let histogram = histogram.to_log_histogram().unwrap();
            let histogram = histogram.borrow();
            let sum       = compute_sum(&histogram);

            assert!(sum == sample_count);
        }

        let hz = continuing_box().borrow().hz();

        assert!(stat.hz == hz as i64);

        let any      = stat.as_any();
        let any_stat = any.downcast_ref::<RunningTime>().unwrap();

        assert!(stat.equals(any_stat));

        // Now set_id() and id() to check equality.

        let expected = 12034; // Something unliklely.

        stat.set_id(expected);

        let any      = stat.as_any_mut();
        let any_stat = any.downcast_ref::<RunningTime>().unwrap();

        assert!(any_stat.id() == expected);
    }

    #[test]
    #[should_panic]
    fn test_record_i64() {
        let     timer = continuing_box();
        let mut stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.record_i64(1);
    }

    #[test]
    #[should_panic]
    fn test_record_f64() {
        let     timer = continuing_box();
        let mut stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.record_f64(1.0);
    }

    #[test]
    #[should_panic]
    fn test_min_f64() {
        let timer = continuing_box();
        let stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.min_f64();
    }

    #[test]
    #[should_panic]
    fn test_max_f64() {
        let timer = continuing_box();
        let stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.max_f64();
    }

    #[test]
    #[should_panic]
    fn test_float_histogram() {
        let timer = continuing_box();
        let stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.float_histogram().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_to_float() {
        let timer = continuing_box();
        let stat  = RunningTime::new("Panic Test", timer, &None);

        let _ = stat.to_float_histogram().unwrap();
    }

    fn test_histogram() {
        let     timer   = continuing_box();
        let mut stat    = RunningTime::new("Panic Test", timer, &None);
        let     samples = 200;

        for i in 1..=samples {
            stat.record_time(i as i64);
        }

        {
            let histogram = stat.to_log_histogram().unwrap();
            let histogram = histogram.borrow();

            let sum = compute_sum(&histogram);

            assert!(sum == samples)
        }

        stat.clear_histogram();

        {
            let histogram = stat.to_log_histogram().unwrap();
            let histogram = histogram.borrow();

            let sum = compute_sum(&histogram);

            assert!(sum == 0)
        }
    }

    fn test_equality() {
        let timer  = continuing_box();
        let stat_1 = RunningTime::new("Equality Test 1", timer, &None);

        let timer  = continuing_box();
        let stat_2 = RunningTime::new("Equality Test 2", timer, &None);

        let stat_3 = Counter::new("Equality Test 2", &None);

        assert!( stat_1.equals(&stat_1));
        assert!(!stat_1.equals(&stat_2));
        assert!(!stat_1.equals(&stat_3));

        let generic = stat_1.generic();
        let recast  = generic.downcast_ref::<RunningTime>().unwrap();

        assert!(stat_1.equals(recast));
    }

    pub struct LargeTimer {
    }

    impl Timer for LargeTimer {
        fn start(&mut self) {
        }

        fn finish(&mut self) -> i64 {
            1
        }

        fn hz(&self) -> u128 {
            u128::MAX
        }
    }

    fn test_large_timer() {
        let mut timer = LargeTimer { };

        timer.start();

        let _ = timer.finish();
    }

    #[test]
    #[should_panic]
    fn test_large_clock() {
        let timer = LargeTimer { };
        let timer = timer_box!(timer);
        let _     = RunningTime::new("Panic Test", timer, &None);
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
                "    Skewness         -4.16336 e-11 ",
                "    Kurtosis         -1.19999 e+0  ",
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
        let mut stats      = RunningTime::new(&name, timer, &print_opts);
        let     samples    = 1000;

        for _i in 1..=samples {
            stats.record_event();
        }

        stats.print();
    }

    #[test]
    fn run_tests() {
        simple_test();
        test_equality();
        test_histogram();
        test_large_timer();
        test_print_output();
    }
}
