//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! 'Rustics' provides a very simple interface for recording events and printing statistics.
//!
//! ## Types
//!
//! * Statistics for Integer Values
//!     * Integer statistics provide basic parameters, like the mean, and a pseudo-log histogram.
//!     * For the pseudo-log histogram, the pseudo-log of a negative number n is defines as 
//!       -log(-n).  The pseudo-log of 0 is defined as 0, and for convenience, the pseudo-log of
//!       -(2^64) is defined as 63.  Logs of positive values are computed by rounding up any
//!       fractional part, so the pseudo-log of 5 is 3.  From the definition, the pseudo-log of
//!       -5 is -3.
//!     * The values can be interpreted as time periods with a given hertz.
//!
//! * Integer statistics types
//!     * RunningInteger
//!         * This type implements a few running statistics for a series of i64 sample values.
//!         * It also provides a pseudo-log histogram.
//!
//!     * RunningWindow
//!         * This type implements a fixed-size window of the last n samples recorded.  Summary
//!           statistics of the window samples are computed on demand.
//!         * It also provides a pseudo-log historgram.  The histogram counts all samples seen,
//!           not just the current window.
//!
//!     * Counter
//!         * This type implements a simple counter that generates no further statistics.  It
//!           can be used for counting events, for example.
//!
//! * Time statistics types
//!     * RunningTime
//!         * This type uses the RunningInteger code to handle time intervals.  Values will be
//!           printed using units of time.
//!
//!     * TimeWindow
//!         * This type uses the IntegerWindow code to handle time intervals.  As with the
//!           RunningTime type, values are printed in units of time.
//!
//! * Hierarchial Statistics
//!     * Hierarchical statistics contain sums of integer statistics instances, and can be
//!       multi-level.  Values are recorded into an integer statistic of some kind.  When
//!       a hierarchical instance receives a request to advance to a new statistics instance,
//!       the current statistic that is collecting data is pushed into a window, and a new
//!       statistics instance will be created to hold any new values to be recorded.
//!
//!     * When a programmable number of statistics instances have been pushed into a window,
//!       these statistics are summed and the sum placed in a high level window.  The
//!       summation is done recursively up to a programmed number of levels.  Each level
//!       has a parameter specifying the number of instances to be summed.
//!
//!     * The lowest level, the one to which statistical data is recorded, can be configured
//!       to push the current instance and start a new one after a certain number of samples
//!       have been recorded, or the advance() method can be invoked to move to a new
//!       statistics instance.
//!
//!     * Each level of statistics has a programmable "live" count that gives the number of
//!       instances that are summed and pushed to the higher level, and a retention window, so
//!       that statistics are kept for some time period after being summed.
//!
//! * Hierarchical statistics types
//!     * Hier
//!         * The Hier struct implements the framework for hierarchical statistics.
//!         * The HierGenerator trait provides the interface for a Rustics implementation
//!           to be usable in a Hier instance.
//!     * IntegerHier
//!         * This struct wraps the RunningInteger type to support the Hier code.  See
//!           "Integer::new_hier" for a simple interface to get going.  The hier.rs test
//!            module also contains sample_usage() and make_hier() functions as examples.
//!
//!     * TimeHier
//!         * TimeHier implements Hier for the RunningTime type. As with IntegerHier,
//!           see "TimeHier::new_hier" for an easy way to make a Hier instance that uses
//!           RunningTime statistics.
//!
//! * Creating Sets
//!     * The "arc_sets" and "rc_sets" modules implement a simple feature allowing the creation of sets
//!       that accept statistics and subsets as members.
//!
//!     * ArcSet
//!         * This type functions as an Arc-based implementation of sets and subsets that can be printed
//!           and cleared on command.
//!
//!     * RcSet
//!         * This type functions as an Rc-based implementation of sets and subsets.  These sets will be
//!           significantly faster than Arc-based sets, but are not thread-safe.
//!
//! * Timers
//!     *  Timer
//!         * This trait is the basic abstract timer.  A timer has a frequency and returns
//!           an integer duration in units of that frequency.  The Timer interface provides
//!           "start" and "finish" methods to measure clock intervals.
//!
//!     *  DurationTimer
//!         * This implementation of a timer uses the Rust "Duration" implementation, which measures
//!           wall clock time.
//!
//!     *  ClockTimer
//!         * This implementation is a wrapper for a simple time counter (trait SimpleClock) that
//!           returns an integer corresponding to the current "time" value.  For example, a cycle
//!           counter could be wrapped to implement a ClockTimer.  This wrapper can be used with a
//!           platform-specific counter such as one of the Linux clock_* functions.  The wrapper
//!           implementation provides the "start" and "finish" methods, along with initialization,
//!           so the wrapped counter can be very simple.
//!
//!     *  SimpleClock
//!         * This trait defines the interface used by ClockTimer to query a clock.
//!
//!     *  Printer
//!         * This trait provides a method to use custom printers.  By default, output from the
//!           print function goes to stdout.
//!         * See StdioPrinter for a very simple sample implementation.  This trait is used
//!           as the default printer by the Rustics code.
//!

use std::sync::Mutex;
use std::sync::Arc;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use time::Timer;

pub mod running_integer;
pub mod running_time;
pub mod integer_window;
pub mod time_window;
pub mod counter;
pub mod arc_sets;
pub mod rc_sets;
pub mod hier;
pub mod window;
pub mod time;
pub mod sum;
pub mod integer_hier;
pub mod time_hier;
pub mod log_histogram;

pub mod printable;

use hier::Hier;
use hier::HierDescriptor;
use hier::HierConfig;
use hier::HierGenerator;
use hier::HierMember;
use hier::HierExporter;
use hier::ExporterRc;
use hier::MemberRc;
use log_histogram::LogHistogram;

pub type HierBox       = Arc<Mutex<Hier>>;
pub type PrinterBox    = Arc<Mutex<dyn Printer>>;
pub type PrinterOption = Option<Arc<Mutex<dyn Printer>>>;
pub type TimerBox      = Rc<RefCell<dyn Timer>>;

/// timer_box_hz() is a helper function just returns the hertz
/// of a timer in a box.  It just saves a bit of typing.

pub fn timer_box_hz(timer:  &TimerBox) -> u128 {
    (**timer).borrow().hz()
}

/// stdout_printer() creates a Printer instance that sends output
/// to stdout.  This is the default type for all statistics types.

pub fn stdout_printer() -> PrinterBox {
    let printer = StdioPrinter::new(StreamKind::Stdout);

    Arc::new(Mutex::new(printer))
}

/// Compute a variance estimator.

pub fn compute_variance(count: u64, moment_2: f64) -> f64 {
    if count < 2 {
        return 0.0;
    }

    let n = count as f64;

    moment_2 / (n - 1.0)
}

/// Compute the sample skewness.
///
/// This formula is from brownmath.com.

pub fn compute_skewness(count: u64, moment_2: f64, moment_3: f64) -> f64 {
    if count < 3 || moment_2 == 0.0 {
        return 0.0;
    }

    assert!(moment_2 > 0.0);

    let n               = count as f64;
    let m3              = moment_3 / n;
    let m2              = moment_2 / n;
    let skewness        = m3 / m2.powf(1.5);
    let correction      = (n * (n - 1.0)).sqrt() / (n - 2.0);

    skewness * correction
}

/// Compute the sample kurtosis estimator.
///
/// This formula is from brownmath.com.

pub fn compute_kurtosis(count: u64, moment_2: f64, moment_4: f64) -> f64 {
    if count < 4 || moment_2 == 0.0 {
        return 0.0;
    }

    assert!(moment_2 > 0.0 && moment_4 >= 0.0);

    let n               = count as f64;
    let kurtosis        = moment_4 / (moment_2.powf(2.0) / n) - 3.0;
    let correction      = (n - 1.0) / ((n - 2.0) * (n - 3.0));
    let kurtosis_factor = (n + 1.0) * kurtosis + 6.0;

    correction * kurtosis_factor
}

/// The make_title() function concatenates two strings, insering the
/// "=>" marker for set hierarchy specification.  It is probably of
/// interest only to implementors of new statistics types.  It does
/// omit the "=?" if the title prefix is empty.

pub fn make_title(title_prefix: &str, title: &str) -> String {
    if title_prefix.is_empty() {
        title.to_string()
    } else {
        let mut full_title = String::from(title_prefix);
        full_title.push_str(" ==> ");
        full_title.push_str(title);
        full_title
    }
}

/// The Printer trait allows users to create custom output types to
/// match their I/O needs.
///
/// An instance of this type is invoked for each line to be printed.
/// The print() member is responsible for adding the newline.

pub trait Printer {
    fn print(&self, output: &str);
}

// Define a printer that will send output to Stdout or Stderr, as
// configured.

/// The StdioPrinter struct is used as the default printer by Rustics.
/// It serves as an example of a very simple Printer implementation.

#[derive(Clone)]
pub struct StdioPrinter {
    which: StreamKind,
}

#[derive(Clone)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

impl StdioPrinter {
    pub fn new(which: StreamKind) -> StdioPrinter {
        StdioPrinter { which }
    }
}

impl Printer for StdioPrinter {
    fn print(&self, output: &str) {
        match self.which {
            StreamKind::Stdout => println!("{}", output),
            StreamKind::Stderr => eprintln!("{}", output),
        }
    }
}

/// The Rustics trait is the main interface for collecting
/// and querying statistics.

pub trait Rustics {
    fn record_i64  (&mut self, sample: i64);  // add an i64 sample
    fn record_f64  (&mut self, sample: f64);  // add an f64 sample -- not implemented
    fn record_event(&mut self             );  // implementation-specific record
    fn record_time (&mut self, sample: i64);  // add a time sample

    fn record_interval(&mut self, timer: &mut TimerBox);
                                             // Add a time sample ending now

    fn name(&self)               -> String;  // a text (UTF-8) name
    fn title(&self)              -> String;  // a text (UTF-8) title to print
    fn class(&self)              -> &str;    // the type of a sample:  "integer", etc
    fn count(&self)              -> u64;     // the current sample count
    fn log_mode(&self)           -> isize;   // the most common pseudo-log
    fn mean(&self)               -> f64;
    fn standard_deviation(&self) -> f64;
    fn variance(&self)           -> f64;
    fn skewness(&self)           -> f64;
    fn kurtosis(&self)           -> f64;

    fn int_extremes(&self)       -> bool;    // does this statistic implement integer extremes?
    fn min_i64(&self)            -> i64;     // return the minimum sample value seen
    fn min_f64(&self)            -> f64;
    fn max_i64(&self)            -> i64;     // return the maximum sample value seen
    fn max_f64(&self)            -> f64;

    fn precompute(&mut self);                // precompute the various statistics for printing
    fn clear(&mut self);                     // clear all the statistics

    // Functions for printing

    fn print     (&self);
    fn print_opts(&self, printer: PrinterOption, title: Option<&str>);

    fn set_title (&mut self, title: &str);

    // For internal use only.

    fn set_id   (&mut self, id: usize      );
    fn id       (&self                     ) -> usize;
    fn equals   (&self, other: &dyn Rustics) -> bool;
    fn generic  (&self                     ) -> &dyn Any;
    fn histogram(&self                     ) -> LogHistogram;
}

/// Histogram defines the trait for using a LogHistogram instance.

pub trait Histogram {
    fn log_histogram(&self) -> LogHistogram;
    fn print_histogram(&self, printer: &mut dyn Printer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use crate::running_time::RunningTime;
    use crate::running_integer::RunningInteger;
    use crate::integer_window::IntegerWindow;
    use crate::time_window::TimeWindow;
    use crate::printable::Printable;
    use crate::log_histogram::pseudo_log_index;

    // This struct is used by other modules.

    pub struct TestPrinter {
        prefix: String,
    }

    impl TestPrinter {
        pub fn new(prefix: &str) -> TestPrinter {
            let prefix = prefix.to_string();

            TestPrinter { prefix }
        }
    }

    impl Printer for TestPrinter {
        fn print(&self, output: &str) {
            println!("{}:  {}", self.prefix, output);
        }
    }

    // Define a testing clock that allows us to define the
    // intervals that the clock returns.  We do this through
    // a global variable, although creating a list might be
    // more rust-like.

    #[derive(Clone, Copy)]
    pub struct TestTimer {
        started: bool,
        ticks:   i64,
        hz:      u128,
    }

    impl TestTimer {
        pub fn new(hz: u128) -> TestTimer {
            let started = false;
            let ticks   = 0;

            TestTimer { started, ticks, hz }
        }

        pub fn new_box(hz: u128) -> Rc<RefCell<TestTimer>> {
            let timer = TestTimer::new(hz);

            Rc::from(RefCell::new(timer))
        }

        pub fn setup_elapsed_time(&mut self, ticks: i64) {
            assert!(ticks >= 0);

            self.started = true;
            self.ticks   = ticks;
        }
    }

    impl Timer for TestTimer {
        fn start(&mut self) {
            assert!(self.started);
        }

        fn finish(&mut self) -> i64 {
            assert!(self.started);
            assert!(self.ticks >= 0);

            self.started = false;
            self.ticks
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    // We need dynamic conversion.

    pub trait TestTimerTrait {
        fn setup(&mut self, ticks: i64);
    }

    impl TestTimerTrait for TestTimer {
        fn setup(&mut self, ticks: i64) {
            self.setup_elapsed_time(ticks);
        }
    }

    pub trait ConverterTrait : Timer + TestTimerTrait {
        fn as_timer(it: Rc<RefCell<Self>>) -> Rc<RefCell<dyn Timer>>;
        fn as_test_timer(it: Rc<RefCell<Self>>) -> Rc<RefCell<dyn TestTimerTrait>>;
    }

    impl ConverterTrait for TestTimer {
        fn as_timer(it: Rc<RefCell<Self>>) -> Rc<RefCell<dyn Timer>> {
            it
        }

        fn as_test_timer(it: Rc<RefCell<Self>>) -> Rc<RefCell<dyn TestTimerTrait>> {
            it
        }
    }

    fn test_test_timer() {
        let hz    = 1_000_000_000;
        let both  = TestTimer::new_box(hz);
        let setup = ConverterTrait::as_test_timer(both.clone());
        let value = ConverterTrait::as_timer(both.clone());

        for i in 1..100 {
            setup.borrow_mut().setup(i);
            assert!(value.borrow_mut().finish() == i);
        }
    }

    // Define a simple timer for testing that just starts at 1000 ticks
    // and then counts up by 1000 ticks for each event interval.  This
    // type is used by other test modules.

    pub fn continuing_timer_increment() -> i64 {
        1000
    }

    pub fn continuing_box() -> TimerBox {
        let hz = 1_000_000_000;

        Rc::from(RefCell::new(ContinuingTimer::new(hz)))
    }

    pub struct ContinuingTimer {
        time:       i64,
        increment:  i64,
        hz:         u128,
    }

    impl ContinuingTimer {
        pub fn new(hz: u128) -> ContinuingTimer {
            let time      = 0;
            let increment = continuing_timer_increment();

            ContinuingTimer { time, increment, hz }
        }
    }

    impl Timer for ContinuingTimer {
        fn start(&mut self) {
            self.time = 0;
        }

        fn finish(&mut self) -> i64 {
            self.time += self.increment;
            self.time
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    pub fn setup_elapsed_time(timer: &mut Rc<RefCell<TestTimer>>, ticks: i64) {
        let mut timer = timer.borrow_mut();

        timer.setup_elapsed_time(ticks);
    }

    // Set up the next interval to be returned.

    pub fn test_running_time() {
        println!("Testing running time statistics.");

        let     hz         = 1_000_000_000;
        let     both       = TestTimer::new_box(hz);
        let     test_timer = ConverterTrait::as_test_timer(both.clone());
        let mut stat_timer = ConverterTrait::as_timer(both.clone());
        let     printer    = Some(stdout_printer());
        let mut time_stat  = RunningTime::new("Test Running Time 1", stat_timer.clone(), printer);

        test_timer.borrow_mut().setup(i64::MAX);
        time_stat.record_event();

        assert!(time_stat.min_i64() == i64::MAX);
        assert!(time_stat.max_i64() == i64::MAX);

        test_timer.borrow_mut().setup(0);
        time_stat.record_event();

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == i64::MAX);

        let mut rng = rand::thread_rng();

        // Let the random number generator run wild.

        for _i in 1..100 {
            let random: i32 = rng.gen();

            let interval =
                if random > 0 {
                    random as i64
                } else if random == 0 {
                    1 as i64
                } else {
                    -(random + 1) as i64
                };

            test_timer.borrow_mut().setup(interval);
            time_stat.record_event();
        }

        println!("test_running_time:  first stats added.");
        time_stat.print();

        println!("test_running_time:  first print done.");

        // Okay, use a more restricted range of times.

        let     printer    = Some(stdout_printer());
        let mut time_stat  = RunningTime::new("Test Running Time 2", stat_timer.clone(), printer);

        let limit = 99;

        for i in 0..limit + 1 {
            let interval = i * i * i;
            test_timer.borrow_mut().setup(interval);

            // Test both record_event and record_interval.

            if i & 1 != 0 {
                time_stat.record_event();
            } else {
                time_stat.record_interval(&mut stat_timer);
            }
        }

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == limit * limit * limit);

        time_stat.print();

        // Get a sample with easily calculated summary statistics

        let     printer   = Some(stdout_printer());
        let mut time_stat = RunningTime::new("Test Time => 1..100", stat_timer.clone(), printer);

        for i in 1..101 {
            test_timer.borrow_mut().setup(i);
            time_stat.record_event();

            assert!(time_stat.max_i64() == i);
        }

        time_stat.print();

        // Cover all the scales.

        let     printer   = Some(stdout_printer());
        let mut time_stat = RunningTime::new("Time => Scale", stat_timer.clone(), printer);

        let mut time    = 1;
        let mut printer = TestPrinter::new("Time Scale Test");

        /* To-do:  create a printer that saves the string for examination.
        let expected_output =
            [
                (  1.000, "ns"     ),
                ( 10.000, "ns"     ),
                (100.000, "ns"     ),
                (  1.000, "us"     ),
                ( 10.000, "us"     ),
                (100.000, "us"     ),
                (  1.000, "ms"     ),
                ( 10.000, "ms"     ),
                (100.000, "ms"     ),
                (  1.000, "second" ),
                ( 10.000, "seconds"),
                (  1.667, "minutes"),
                ( 16.667, "minutes"),
                (  2.778, "hours"  ),
                (  1.157, "days"   )
            ];
        */

        for i in 1..16 {
            let elapsed = i * 100;

            test_timer.borrow_mut().setup(elapsed);

            if i & 1 != 0 {
                time_stat.record_event();
            } else {
                time_stat.record_interval(&mut stat_timer);
            }

            assert!(time_stat.max_i64() == elapsed);

            let header = format!("{}", Printable::commas_i64(time));
            Printable::print_time(&header, time as f64, hz as i64, &mut printer);

            time *= 10;
        }

        time_stat.print();
    }

    fn test_time_window() {
        println!("Testing time windows.");

        let     hz        = 1_000_000_000;
        let     both       = TestTimer::new_box(hz);
        let     test_timer = ConverterTrait::as_test_timer(both.clone());
        let     stat_timer = ConverterTrait::as_timer(both.clone());
        let mut time_stat = TimeWindow::new("Test Time Window 1", 50, stat_timer.clone(), None);

        assert!(time_stat.class() == "time");

        test_timer.borrow_mut().setup(i64::MAX);
        time_stat.record_event();

        assert!(time_stat.min_i64() == i64::MAX);
        assert!(time_stat.max_i64() == i64::MAX);

        test_timer.borrow_mut().setup(0);
        time_stat.record_event();

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == i64::MAX);

        let mut rng = rand::thread_rng();

        // Let the random number generator run wild.

        for _i in 1..100 {
            let random: i32 = rng.gen();

            let interval =
                if random >= 0 {
                    random as i64
                } else {
                    -(random + 1) as i64
                };

            test_timer.borrow_mut().setup(interval);
            time_stat.record_event();
        }

        time_stat.print();

        // Okay, use a more restricted range of times.

        let     printer   = Some(stdout_printer());
        let mut time_stat = RunningTime::new("Test Time Window 2", stat_timer.clone(), printer);

        assert!(time_stat.class() == "time");

        let limit = 99;

        for i in 0..limit + 1 {
            let interval = i * i * i;

            test_timer.borrow_mut().setup(interval);
            time_stat.record_event();
        }

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == limit * limit * limit);

        time_stat.print();

        // Get a sample with easily calculated summary statistics

        let     printer   = Some(stdout_printer());
        let mut time_stat = RunningTime::new("Time Window => 1..100", stat_timer.clone(), printer);

        for i in 1..101 {
            test_timer.borrow_mut().setup(i);
            time_stat.record_event();

            assert!(time_stat.max_i64() == i);
        }

        let float_count = time_stat.count() as f64;
        let sum         = (100 * (100 + 1) / 2) as f64;
        let mean        = sum / float_count;

        assert!(time_stat.mean() == mean);

        time_stat.print();

        // Cover all the scales.

        let mut timer     = TestTimer::new_box(hz);
        let     printer   = Some(stdout_printer());
        let mut time_stat = RunningTime::new("Time => Scale", timer.clone(), printer);

        let mut time    = 1;
        let     printer = &mut StdioPrinter::new(StreamKind::Stdout);

        for _i in 1..16 {
            setup_elapsed_time(&mut timer, time);
            time_stat.record_event();
            let header = format!("{} => ", Printable::commas_i64(time));
            Printable::print_time(&header, time as f64, hz as i64, printer);

            time *= 10;
        }

        time_stat.print();
    }

    fn run_all_histo_tests() {
        let     timer           = continuing_box();
        let mut running_integer = RunningInteger::new("RunningInteger", None);
        let mut running_time    = RunningTime::new   ("RunningTime",    timer.clone(), None);
        let mut integer_window  = IntegerWindow::new ("IntegerWindow",  100, None);
        let mut time_window     = TimeWindow::new    ("TimeWindow",     100, timer.clone(), None);

        run_histogram_tests(&mut running_integer);
        run_histogram_tests(&mut running_time   );
        run_histogram_tests(&mut integer_window );
        run_histogram_tests(&mut time_window    );
    }

    // This test is used by other modules.

    pub fn run_histogram_tests(rustics: &mut dyn Rustics) {
        let values = [ -300, 0, 300, 1300, 2300, i64::MIN, i64::MAX ];

        for value in values {
            test_histogram(rustics, value);
        }
    }

    // Put some samples into the instance, then check the
    // contents.

    fn test_histogram(rustics: &mut dyn Rustics, mut value: i64) {
        rustics.clear();

        assert!(rustics.count() == 0);

        // Check the histogram...

        let histogram = rustics.histogram();

        for item in histogram.negative {
            assert!(item == 0);
        }

        for item in histogram.positive {
            assert!(item == 0);
        }

        // Time instances only get positive values...  Avoid overflow
        // when negating and adding.  Consider MAX and MIN...

        if rustics.class() == "time" && value <= 0 {
            //value = std::cmp::max((value + 2).abs() + 1, 1);
            value = (value + 2).abs() + 1;
        }

        // Record the values and count the events.

        let mut events = 0;

        for _i in 0..100 {
            if rustics.class() == "time" {
                rustics.record_time(value);
            } else {
                rustics.record_i64(value);
            }

            events += 1;
        }

        // Check that the data seems sane.

        assert!(rustics.standard_deviation() == 0.0);

        assert!(rustics.mean()     == value as f64);
        assert!(rustics.variance() == 0.0);
        assert!(rustics.skewness() == 0.0);
        assert!(rustics.kurtosis() == 0.0);

        // Check that the histogram matches expectation.
        let histogram       = rustics.histogram();
        let log_mode_index  = pseudo_log_index(value);

        for i in 0..histogram.negative.len() {
            if value < 0 && i == log_mode_index {
                assert!(histogram.negative[i] == events);
            } else {
                assert!(histogram.negative[i] == 0);
            }
        }

        for i in 0..histogram.positive.len() {
            if value >= 0 && i == log_mode_index {
                assert!(histogram.positive[i] == events);
            } else {
                assert!(histogram.positive[i] == 0);
            }
        }
    }

    #[test]
    pub fn run_tests_dyn() {
        test_time_window();
        test_running_time();
        run_all_histo_tests();
        test_test_timer();
    }
}
