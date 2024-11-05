//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//! 'Rustics' provides a very simple interface for recording events and printing statistics.
//!
//! Many of the module comments contain examples of usage.
//!
//! ## Types
//!
//! * Statistics for Integer Samples
//!     * Integer statistics provide basic parameters, like the mean, and a pseudo-log histogram.
//!
//!     * For the pseudo-log histogram, the pseudo-log of a negative number n is defines as
//!       -log(-n).  The pseudo-log of 0 is defined as 0.  Logs of positive values are computed by
//!       rounding up any fractional part, so the pseudo-log of 5 is 3.  From the definition, the
//!       pseudo-log of -5 is -3.
//!
//!     * The time-based statistics work on time samples measured in integer ticks.
//!
//! * Integer statistics types
//!     * RunningInteger
//!         * RunningInteger implements running statistics for a series of i64 sample values.
//!         * It also provides a pseudo-log histogram of the samples.
//!
//!     * RunningWindow
//!         * IntegerWindow implements a fixed-size window of the last n samples recorded.  Summary
//!           statistics of the window samples are computed on demand.
//!         * It also provides a pseudo-log historgram.  The histogram counts all samples seen,
//!           not just the current window.
//!
//!     * Counter
//!         * This type implements a simple counter that generates no further statistics.  It can be
//!           used for counting events, for example.
//!
//! * Time statistics types
//!     * RunningTime
//!         * This type uses the RunningInteger code to handle time intervals.  Values are printed
//!           using units of time.
//!
//!     * TimeWindow
//!         * This type uses the IntegerWindow code to handle time intervals.  As with the
//!           RunningTime type, values are printed in units of time.
//! * Floating point statistics type
//!     * RunningFloat
//!         * This type uses samples of type f64.  It is otherwise similar to
//!           RunningInteger.  It uses a coarser pseudo-log function than the
//!           integer statistics.  See FloatHistogram for details.
//!
//!     * FloatWindow
//!         * This type uses samples of type f64.  It is otherwise similar to
//!           IntegerWindow.  It creates a histogram using FloatHistogram.
//!
//! * Hierarchial Statistics
//!     * The Hier struct implements an ordered set of vectors of Rustics instances.
//!
//!     * Each element of the set is implemented using a Window instance, which holds an ordered
//!       set of Rustics instances.  As new instances are added to a level, older ones are
//!       deleted as necessary to obey a configurable size limit.  Each window repesents a level
//!       of the hierarchical statistics.
//!
//!     * Level 0 contains Rustics instances that have been used to recorded data samples, and the
//!       upper levels are sums of Rustics instances from lower levels.
//!
//!     * New samples are recorded into the newest Rustics instance at level 0.
//!
//!     * When a given number of Rustics instances have been pushed into a window, these instances
//!       are summed and the sum placed in the next higher level window.
//!
//!     * Level 0 can be configured to push the current Rustics instance and start a new one
//!       after a given number of samples have been recorded, or the user can invoke the advance()
//!       method to push a new Rustics instance into the level 0 window.
//!
//!     * Each level also has retention limt and will retained a set of the last n instances
//!       pushed onto this level, even if those instances have been summed into a higher-level
//!       instance.
//!
//! * Hierarchical statistics types
//!     * Hier
//!         * The Hier struct provies framework code for hierarchical statistics.  After creating
//!           a Hier instance, most users will use this interface or the Rustics interface to
//!           interact with this type.  For example, data is recorded into a Hier instance
//!           by invoking Rustics methods directly on the Hier instance itself.
//!
//!         * The HierGenerator trait is implemented to allow the Hier implementation to use a
//!           specific Rustics implementation, like RunningInteger or RunningTime.  Most users
//!           will not use this type directly.
//!
//!     * IntegerHier
//!         * This struct wraps the RunningInteger type to support the Hier code.  See
//!           "Integer::new_hier" for a simple interface to create a Hier instance using
//!           RunningInteger for statistics collection.  The integer_hier and hier.rs test module
//!           also contains sample_usage() and make_hier() functions as examples.
//!
//!     * TimeHier
//!         * TimeHier implements Hier for the RunningTime type. As with IntegerHier, see
//!           "TimeHier::new_hier" for an easy way to make a Hier instance that uses RunningTime
//!           as the statistics type.
//!
//! * Creating Sets
//!     * The "arc_sets" and "rc_sets" modules implement a simple feature allowing the creation
//!       of sets that accept Rustics instances and other sets as members.  Sets can be printed and
//!       cleared recursiely by invoking a method on the topmost set.
//!
//!     * ArcSet
//!         * This type provides an Arc-based implementation that is thread-safe.
//!
//!     * RcSet
//!         * This type implements an Rc-based implementation of sets.  These sets are
//!           faster than Arc-based sets, but are not thread-safe.
//!
//! * Timers
//!     *  Timer
//!         * This trait is the basic abstract timer.  A timer has a frequency and returns
//!           an integer duration in units of that frequency.  The Timer interface provides
//!           start() and finish() methods to measure clock intervals.
//!
//!     *  DurationTimer
//!         * This implementation of Timer uses the Rust "Duration" implementation, which measures
//!           wall clock time.
//!
//!     *  ClockTimer
//!         * This Timer implementation is a wrapper for a simple time counter (trait SimpleClock)
//!           that returns an integer corresponding to the current "time" value.  For example, a
//!           cycle counter like rdtsc on Intel could be wrapped to implement a ClockTimer.
//!
//!     *  SimpleClock
//!         * This trait defines the interface used by ClockTimer to query a user-defined clock.
//!
//!         * Clock values are returned as an integer tick count.
//!
//!         * SimpleClock implementation provide a hz() member to return the hertz the ClockTimer
//!           layer.
//!
//! * Printing
//!     *  Printer
//!         * This trait provides a method to use custom printers.  By default, output from the
//!           printing function goes to stdout.
//!
//!         * See StdioPrinter for a very simple sample implementation.  This trait is used
//!           as the default printer by the Rustics code.
//!
//!     *  Printable
//!         * Printable provides standard formatting for printing data and some support functions
//!           for nicer output, like time values scaled to human-understanble forms and integers
//!           with commas.  It is of interest mostly to developers creating new Rustics
//!           implementations.
//!

use std::sync::Mutex;
use std::sync::Arc;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::default::Default;

use time::Timer;

pub mod running_integer;
pub mod integer_window;
pub mod integer_hier;

pub mod running_time;
pub mod time_window;
pub mod time_hier;

pub mod running_float;
pub mod float_window;
pub mod float_hier;

pub mod counter;
pub mod arc_sets;
pub mod rc_sets;
pub mod hier;
pub mod window;
pub mod time;
pub mod merge;
pub mod sum;
pub mod log_histogram;
pub mod float_histogram;

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
use float_histogram::FloatHistogram;
use float_histogram::HistoOpts;
use printable::Printable;

pub type HierBox            = Arc<Mutex<Hier>>;
pub type PrinterBox         = Rc<RefCell<dyn Printer>>;
// pub type PrinterBox         = Arc<Mutex<dyn Printer>>;
pub type PrinterOption      = Option<PrinterBox>;
pub type TitleOption        = Option<String>;
pub type UnitsOption        = Option<Units>;
pub type HistoOption        = Option<HistoOpts>;
pub type TimerBox           = Rc<RefCell<dyn Timer>>;
pub type PrintOption        = Option<PrintOpts>;
pub type LogHistogramBox    = Rc<RefCell<LogHistogram>>;
pub type FloatHistogramBox  = Rc<RefCell<FloatHistogram>>;

pub fn to_mantissa(input: f64) -> i64 {
    let mantissa_size = 52;

    let bits = input.to_bits();
    let mask = (1_u64 << mantissa_size) - 1;

    (bits & mask) as i64
}

pub fn max_exponent() -> isize {
    1023
}

pub fn max_biased_exponent() -> isize {
    max_exponent() + exponent_bias()
}

pub fn min_exponent() -> isize {
    -1022
}

pub fn exponent_bias() -> isize {
    1023
}

pub fn sign(input: f64) -> isize {
    if input.to_bits() & (1_u64 << 63) != 0 {
        -1
    } else {
        1
    }
}

pub fn is_zero(input: f64) -> bool {
    input == 0.0    // -0.0 == 0 per IEEE definition
}

pub fn biased_exponent(input: f64) -> isize {
    if input.is_nan() {
        return 0;
    }

    if input.is_infinite() {
        return max_exponent();
    }

    let mantissa_size = 52;
    let exponent_size = 11;

    let bits     = input.to_bits();
    let bits     = bits >> mantissa_size;
    let mask     = (1_u64 << exponent_size) - 1;
    let exponent = (bits & mask) as i64;

    exponent as isize
}

pub fn min_f64(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

pub fn max_f64(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

/// timer_box_hz() is a helper function just returns the hertz
/// of a timer in a box.  It just saves a bit of typing.

pub fn timer_box_hz(timer:  &TimerBox) -> u128 {
    (**timer).borrow().hz()
}

/// stdout_printer() creates a Printer instance that sends output
/// to stdout.  This is the default type for all statistics types.

pub fn stdout_printer() -> PrinterBox {
    let printer = StdioPrinter::new(StreamKind::Stdout);

    printer_box!(printer)
}

/// Computes a variance estimator.

pub fn compute_variance(count: u64, moment_2: f64) -> f64 {
    if count < 2 {
        return 0.0;
    }

    let n = count as f64;

    moment_2 / (n - 1.0)
}

pub struct StatisticsData {
    pub n:        f64,
    pub sum:      f64,
    pub squares:  f64,
    pub cubes:    f64,
    pub quads:    f64,
}

pub struct Statistics {
    pub mean:       f64,
    pub moment_2:   f64,
    pub moment_4:   f64,
}

/// Computes the second and fourth moments about the mean
/// given the values in StatisticsData.  This is used
/// when merging multiple Rustics instances for upper-
/// level HierMember instances.
///
/// The formulae are derived by applying the binomial
/// theorem to the formulae for the various moments about
/// the mean.

pub fn compute_statistics(data: StatisticsData) -> Statistics {
    let n       = data.n;
    let sum     = data.sum;
    let squares = data.squares;
    let cubes   = data.cubes;
    let quads   = data.quads;

    let mean    = data.sum / n;
    let mean_1  = mean;
    let mean_2  = mean.powi(2);
    let mean_3  = mean.powi(3);
    let mean_4  = mean.powi(4);

    let moment_2 =
                       squares
      - 2.0 * mean_1 * sum
      +       mean_2 * n;

    let moment_4 =
                       quads
      - 4.0 * mean_1 * cubes
      + 6.0 * mean_2 * squares
      - 4.0 * mean_3 * sum
      +       mean_4 * n;
    
    Statistics { mean, moment_2, moment_4 }
}

pub struct RecoverData {
    pub n:          f64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub cubes:      f64,
    pub moment_4:   f64,
}

/// This routine converts the data in RecoverData
/// into estimators of the sum of the squares and
/// 4th power of each sample.  This is used when
/// merging Rustics instances for a Hier instance.
///
/// The formulae are derived by appllying the binomial
/// theorem to the definition of the various moments
/// about the mean

pub fn recover(data: RecoverData) -> (f64, f64) {
    let n        = data.n;
    let mean     = data.mean;
    let moment_2 = data.moment_2;
    let cubes    = data.cubes;
    let moment_4 = data.moment_4;

    let sum      = n * mean;
    let squares  = moment_2 + 2.0 * mean * sum - n * mean.powi(2);

    let mean_1   = mean;
    let mean_2   = mean.powi(2);
    let mean_3   = mean.powi(3);
    let mean_4   = mean.powi(4);

    let quads = 
                   moment_4              
          + (4.0 * cubes    * mean_1)
          - (6.0 * squares  * mean_2)
          + (4.0 * sum      * mean_3)
          - (      n        * mean_4);

    (squares, quads)
}

pub struct EstimateData {
    pub n:          f64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub cubes:      f64,
}

/// Estimate moment 3 about the mean given the data in
/// EstimateData.
///
/// I know of no good way to keep a running estimate of
/// the 3rd moment about the mean, so this is the best
/// I can do.  The equations for the squares and the
/// third moment about the mean are from the binomial
/// theorem applied to the definition of those moments.

pub fn estimate_moment_3(data: EstimateData) -> f64 {
    let n         = data.n;
    let mean      = data.mean;
    let cubes     = data.cubes;
    let moment_2  = data.moment_2;
    let sum       = n * mean;

    // Estimate the sums of the squares of each sample.

    let squares = 
        moment_2
      + 2.0 * sum * mean
      -       n   * mean.powi(2);

    cubes - (3.0 * squares * mean) + 3.0 * (sum * mean.powi(2)) - n * mean.powi(3)
}

/// Computes the sample skewness.
///
/// This formula is from brownmath.com.

pub fn compute_skewness(count: u64, moment_2: f64, moment_3: f64) -> f64 {
    if count < 3 || moment_2 == 0.0 {
        return 0.0;
    }


    // Deal with floating point non-finite values.

    // For debugging new statistics types.
    //
    //assert!(moment_2 > 0.0);

    if moment_2 <= 0.0 {
        return 0.0;
    }

    let n               = count as f64;
    let m3              = moment_3 / n;
    let m2              = moment_2 / n;
    let skewness        = m3 / m2.powf(1.5);
    let correction      = (n * (n - 1.0)).sqrt() / (n - 2.0);

    skewness * correction
}

/// Computes the sample kurtosis estimator.
///
/// This formula is from brownmath.com.

pub fn compute_kurtosis(count: u64, moment_2: f64, moment_4: f64) -> f64 {
    if count < 4 || moment_2 == 0.0 {
        return 0.0;
    }

    // Deal with floating point non-finite values.

    // For debugging new statistics types.
    //
    // assert!(moment_2 > 0.0 && moment_4 >= 0.0);

    if moment_2 <= 0.0 || moment_4 <= 0.0 {
        return 0.0;
    }

    let n               = count as f64;
    let kurtosis        = moment_4 / (moment_2.powf(2.0) / n) - 3.0;
    let correction      = (n - 1.0) / ((n - 2.0) * (n - 3.0));
    let kurtosis_factor = (n + 1.0) * kurtosis + 6.0;

    correction * kurtosis_factor
}

/// The make_title() function concatenates two strings, inserting the
/// "=>" marker for set hierarchy specification.  It is probably of
/// interest only to implementors of new statistics types.  It does
/// omit the "=>" if the title prefix is empty.

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

/// Defines printable strings for a value's units.

#[derive(Clone)]
pub struct Units {
    pub singular:   String,
    pub plural:     String,
}

impl Units {
    /// Return an Units struct with empty strings.  This is used
    /// internally when printing without units.

    pub fn empty() -> Units {
        let singular = "".to_string();
        let plural   = "".to_string();

        Units { singular, plural }
    }

    pub fn new(singular: &str, plural: &str) -> Units {
        let singular = singular.to_string();
        let plural   = plural  .to_string();

        Units { singular, plural }
    }
}

impl Default for Units {
    fn default() -> Self {
        Units::empty()
    }
}

#[derive(Clone)]
pub struct PrintOpts {
    pub printer:     PrinterOption,
    pub title:       TitleOption,
    pub units:       UnitsOption,
    pub histo_opts:  HistoOption,
}

/// The Printer trait allows users to create custom output types to
/// match their I/O needs.
///
/// An instance of this type is invoked for each line to be printed.
/// The print() member is responsible for adding the newline.

pub trait Printer {
    /// Prints a line of output.  The print method itself must append
    /// the newline.

    fn print(&mut self, output: &str);
}

/// The printer macro converts a PrinterBox into borrowed or locked
/// object for printing.

#[macro_export]
macro_rules! printer { ($x:expr) => { &mut *$x.borrow_mut() } }

/// The printer_box macro converts a Printer instance into the
/// shareable form, currently Rc<RefCell<Printer>>.

#[macro_export]
macro_rules! printer_box { ($x:expr) => { Rc::from(RefCell::new($x)) } }

// These macros work with Arc and Mutex
//
// #[macro_export]
// macro_rules! printer { ($x:expr) => { &mut *$x.lock().unwrap() } }
//
// #[macro_export]
// macro_rules! printer_box { ($x:expr) => { Arc::from(Mutex::new($x)) } }

pub fn parse_printer(print_opts: &PrintOption) -> PrinterBox {
    match print_opts {
        Some(print_opts) => {
            match &print_opts.printer {
                Some(printer) => { printer.clone()  }
                None          => { stdout_printer() }
            }
        }

        None => { stdout_printer() }
    }
}

pub fn parse_title(print_opts: &PrintOption, name: &str) -> String {
    match print_opts {
        Some(print_opts) => {
            match &print_opts.title {
                Some(title) => { title.clone()    }
                None        => { name.to_string() }
            }
        }

        None => { name.to_string() }
    }
}

pub fn parse_histo_opts(print_opts: &PrintOption) -> HistoOpts {
    match print_opts {
        Some(print_opts) => {
            match &print_opts.histo_opts {
                Some(histo_opts) => { *histo_opts          }
                None             => { HistoOpts::default() }
            }
        }

        None => { HistoOpts::default()  }
    }
}

pub fn parse_units(print_opts: &PrintOption) -> Units {
    match print_opts {
        Some(print_opts) => {
            match &print_opts.units {
                Some(units) => { units.clone()    }
                None        => { Units::default() }
            }
        }

        None => { Units::default()  }
    }
}

pub fn parse_print_opts(print_opts: &PrintOption, name: &str) -> (PrinterBox, String, Units, HistoOpts) {
    let printer;
    let title;
    let units;
    let histo_opts;

    match print_opts {
        Some(print_opts) => {
            printer =
                match &print_opts.printer {
                    Some(printer) => { printer.clone()  }
                    None          => { stdout_printer() }
                };

            title =
                match &print_opts.title {
                    Some(title) => { title.clone() }
                    None        => { name.to_string()   }
                };

            units =
                match &print_opts.units {
                    Some(units) => { units.clone() }

                    None => { Units::default() }
                };

            histo_opts =
                match &print_opts.histo_opts {
                    Some(histo_opts) => { *histo_opts          }
                    None             => { HistoOpts::default() }
                }
        }

        None => {
            printer    = stdout_printer();
            title      = name.to_string();
            units      = Units::default();
            histo_opts = HistoOpts::default();
        }
    }

    let title = title.to_string();

    (printer, title, units, histo_opts)
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
    fn print(&mut self, output: &str) {
        match self.which {
            StreamKind::Stdout => println! ("{}", output),
            StreamKind::Stderr => eprintln!("{}", output),
        }
    }
}

/// The Rustics trait is the main interface for collecting
/// and querying statistics.

pub trait Rustics {
    /// Records an i64 sample, if allowed by the implementation.
    /// Time-based statistics do not support this method.

    fn record_i64(&mut self, sample: i64);

    /// Records an f64 value.

    fn record_f64(&mut self, sample: f64);

    /// Records an event.  This method is implementation-specific
    /// in meaning.  For the Counter type, it is is equivalent to
    /// record_i64(1).
    ///
    /// The other integer types, e.g., RunningInteger do not support this call.
    ///
    /// For time statistics, it reads the internal timer (provided by the user
    /// to the constructer) for the instance to determine the time interval in
    /// ticks.  The timer is then restarted for the next record_event call.
    ///
    /// The record_event_report is the same as record_event, but it
    /// returns the value that is recorded.  This is used by the Hier
    /// code to implement its window function.
    ///

    fn record_event       (&mut self);         // implementation-specific record
    fn record_event_report(&mut self) -> i64;  // implementation-specific record

    /// Records a time in ticks.  This method will panic if the
    /// underlying type is not a time statistic.

    fn record_time(&mut self, sample: i64);  // add a time sample

    /// Records a time interval by reading the given TimerBox instance.
    /// This method will panic if the underlying type is not a
    /// time statistic.

    fn record_interval(&mut self, timer: &mut TimerBox);
                                             // Add a time sample ending now

    /// Returns the name passed on instance creation.

    fn name(&self) -> String;

    /// Returns the default title used for printing.  The Rc and ArcSet
    /// implementation create hierarchical titles for members of the set.
    /// This function can be used to retrieve them.

    fn title(&self)-> String;

    /// Returns the "class" of the statistic.  Currently, "integer" and
    /// "time" classes exist.

    fn class(&self) -> &str;

    /// Returns the count of samples used to create the summary statistics
    /// like the mean.

    fn count(&self) -> u64;

    /// Returns the most common pseudo-log seen in the data samples.  This
    /// method is supported only for integer and time types.

    fn log_mode(&self) -> isize;

    /// Returns the mean of the samples in the instance.

    fn mean(&self) -> f64;

    /// Returns the standard deviation of the samples in the instance.

    fn standard_deviation(&self) -> f64;

    /// Returns the variance of the samples in the instance.

    fn variance(&self) -> f64;

    /// Returns the skewness of the samples in the instance.

    fn skewness(&self) -> f64;

    /// Returns the kurtosis of the samples in the instance.

    fn kurtosis(&self) -> f64;

    /// Returns a boolean indicating whether the underlying type supports
    /// the min_i64() and max_i64() methods.

    fn int_extremes(&self) -> bool;

    /// Returns a boolean indicating whether the underlying type supports
    /// the min_f64() and max_f64() methods.

    fn float_extremes(&self) -> bool;

    /// Returns the minimum of the sample space for an integer
    /// or time type.  Time statistics return a value in ticks.

    fn min_i64(&self) -> i64;

    /// Returns the minimum of the sample space for an f64 type,
    /// although no implementations currently exist.

    fn min_f64(&self) -> f64;

    /// Returns the maximum of the sample space for an integer
    /// or time type.  Time statistics return a value in ticks.

    fn max_i64(&self) -> i64;

    /// Returns the maximum of the sample space for an f64 type,
    /// although no implementations currently exist.

    fn max_f64(&self) -> f64;

    /// Precomputes the summary data of the samples.  This is
    /// useful when implementing custom print functions or querying
    /// multiple summary statistics like the mean or skewness.
    /// The window statistics will cache the result of data
    /// analysis so it need not be redone each time a summary
    /// statistic is retrieved.

    fn precompute(&mut self);

    /// Clears the data in the statistic.

    fn clear(&mut self);

    // Functions for printing

    fn print     (&self);
    fn print_opts(&self, printer: PrinterOption, title: Option<&str>);

    fn set_title (&mut self, title: &str);

    /// Returns an Rc<...> for the histogram if possible.

    fn log_histogram  (&self) -> Option<LogHistogramBox  >;
    fn float_histogram(&self) -> Option<FloatHistogramBox>;

    // For internal use.

    fn set_id (&mut self, id: usize      );
    fn id     (&self                     ) -> usize;
    fn equals (&self, other: &dyn Rustics) -> bool;
    fn generic(&self                     ) -> &dyn Any;

    fn export_stats(&self) -> ExportStats;
}

pub struct ExportStats {
    pub printable:          Printable,
    pub log_histogram:      Option<LogHistogramBox>,
    pub float_histogram:    Option<FloatHistogramBox>,
}

/// Histogram defines the trait for using a LogHistogram instance.

pub trait Histogram {

    /// Prints the histogram on the given Printer instance.

    fn print_histogram(&self, printer: &mut dyn Printer);

    /// Clear the histogram data.

    fn clear_histogram(&mut self);

    /// Convert the pointer to histogram types if possible.

    fn to_log_histogram  (&self) -> Option<LogHistogramBox>;
    fn to_float_histogram(&self) -> Option<FloatHistogramBox>;
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

    // This function is shared.

    pub fn bytes() -> Option<Units> {
        let singular = "byte".to_string();
        let plural   = "bytes".to_string();

        Some(Units { singular, plural })
    }

    pub fn compute_sum(histogram: &LogHistogram) -> i64 {
        let mut sum = 0;

        for sample in histogram.positive.iter() {
            sum += *sample;
        }

        for sample in histogram.negative.iter() {
            sum += *sample;
        }

        sum as i64
    }

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
        fn print(&mut self, output: &str) {
            println!("{}:  {}", self.prefix, output);
        }
    }

    pub struct CheckPrinter {
        expected:         Vec<String>,
        current:          usize,
        fail_on_overage:  bool,
    }

    impl CheckPrinter {
        pub fn new(expected: &[&str], fail_on_overage: bool) -> CheckPrinter {
            let expected: Vec<String> = expected.iter().map(|x| (*x).into()).collect();
            let current  = 0;

            CheckPrinter { expected, current, fail_on_overage }
        }
    }

    pub fn check_printer_box(expected: &[&str], fail_on_overage: bool) -> PrinterBox {
        let printer = CheckPrinter::new(expected, fail_on_overage);

        printer_box!(printer)
    }

    impl Printer for CheckPrinter {
        fn print(&mut self, output: &str) {
            if self.current < self.expected.len() {
                let expected = self.expected[self.current].clone();

                println!("CheckPrinter:");
                println!("    got      \"{}\"", output);
                println!("    expected \"{}\"", expected);

                assert!(expected == *output);
                self.current += 1;
            } else if self.fail_on_overage {
                panic!("CheckPrinter:  too many lines");
            } else {
                println!(" *** CheckPrinter: ignoring extra lines");
            }

            println!("{}", output);
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
        let mut time_stat  = RunningTime::new("Test Running Time 1", stat_timer.clone(), &None);

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

        let mut random: i32 = 0; // make sure that we test zero.

        for _i in 1..100 {

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
            random = rng.gen();
        }

        println!("test_running_time:  first stats added.");
        time_stat.print();

        println!("test_running_time:  first print done.");

        // Okay, use a more restricted range of times.

        let mut time_stat = RunningTime::new("Test Running Time 2", stat_timer.clone(), &None);

        let limit = 99;

        for i in 0..=limit {
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

        // Get a sample with easily calculated summary statistics.

        let mut time_stat = RunningTime::new("Test Time => 1..100", stat_timer.clone(), &None);

        for i in 1..101 {
            test_timer.borrow_mut().setup(i);
            time_stat.record_event();

            assert!(time_stat.max_i64() == i);
        }

        time_stat.print();

        // Cover all the scales.

        let mut time_stat = RunningTime::new("Time => Scale", stat_timer.clone(), &None);

        let mut time    = 1;
        let mut printer = TestPrinter::new("Time Scale Test");


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

    fn test_time_printing() {
        let hz = 1_000_000_000;

        let ns     =    1;
        let us     = 1000 * ns;
        let ms     = 1000 * us;
        let second = 1000 * ms;
        let minute =   60 * second;
        let hour   =   60 * minute;
        let day    =   24 * hour;

        let values =
            [
                  1_u64,
                 10 * ns,
                100 * ns,
                  1 * us,
                 10 * us,
                100 * us,
                  1 * ms,
                 10 * ms,
                100 * ms,
                  1 * second,
                 10 * second,
                  1 * minute,
                 16 * minute,
                  2 * hour,
                  1 * day,
                  2 * day,

               1175 * day    / 1000,
               1001 * second / 1000
            ];

        let expected_output =
            [
                  "    >                   1.000 ns",
                  "    >                  10.000 ns",
                  "    >                 100.000 ns",
                  "    >                   1.000 us",
                  "    >                  10.000 us",
                  "    >                 100.000 us",
                  "    >                   1.000 ms",
                  "    >                  10.000 ms",
                  "    >                 100.000 ms",
                  "    >                   1.000 second",
                  "    >                  10.000 seconds",
                  "    >                   1.000 minute",
                  "    >                  16.000 minutes",
                  "    >                   2.000 hours",
                  "    >                   1.000 day",
                  "    >                   2.000 days",
                  "    >                   1.175 days",
                  "    >                   1.001 seconds"
            ];

        let mut check_printer = CheckPrinter::new(&expected_output, true);

        for i in 0..values.len() {
            println!("test_time_printing:  value {}, expect {}", values[i], expected_output[i]);
            Printable::print_time(">", values[i] as f64, hz, &mut check_printer);
        }
    }

    fn test_time_window() {
        println!("Testing time windows.");

        let     hz        = 1_000_000_000;
        let     both       = TestTimer::new_box(hz);
        let     test_timer = ConverterTrait::as_test_timer(both.clone());
        let     stat_timer = ConverterTrait::as_timer(both.clone());

        let mut time_stat =
            TimeWindow::new("Test Time Window 1", 50, stat_timer.clone(), &None);

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

        let mut time_stat = RunningTime::new("Test Time Window 2", stat_timer.clone(), &None);

        assert!(time_stat.class() == "time");

        let limit = 99;

        for i in 0..=limit {
            let interval = i * i * i;

            test_timer.borrow_mut().setup(interval);
            time_stat.record_event();
        }

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == limit * limit * limit);

        time_stat.print();

        // Get a sample with easily calculated summary statistics.

        let mut time_stat = RunningTime::new("Time Window => 1..100", stat_timer.clone(), &None);

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
        let mut time_stat = RunningTime::new("Time => Scale", timer.clone(), &None);

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
        let mut running_integer = RunningInteger::new("RunningInteger",                     &None);
        let mut running_time    = RunningTime::new   ("RunningTime",    timer.clone(),      &None);
        let mut integer_window  = IntegerWindow::new ("IntegerWindow",  100,                &None);
        let mut time_window     = TimeWindow::new    ("TimeWindow",     100, timer.clone(), &None);

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

        // Check the histogram...  We need to lose the borrow before
        // calling the record_* functions.

        {
            let histogram = rustics.log_histogram().unwrap();
            let histogram = histogram.borrow();

            for item in histogram.negative {
                assert!(item == 0);
            }

            for item in histogram.positive {
                assert!(item == 0);
            }
        }

        // Time instances only get positive values...  Avoid overflow
        // when negating and adding.  Consider MAX and MIN...

        if rustics.class() == "time" && value <= 0 {
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
        let histogram       = rustics.log_histogram().unwrap();
        let histogram       = histogram.borrow();
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

    fn test_float_functions() {
        let value = 1.0;

        let mantissa = to_mantissa(value);
        assert!(mantissa == 0);

        let value = max_biased_exponent();
        assert!(value == 2046);

        let value = -0.0;
        assert!(is_zero(value));

        let value =  0.0;
        assert!(is_zero(value));

        // Test NaN arguments.

        let value  = f64::NAN;
        let value2 = 1.0;

        assert!(biased_exponent(value) == 0);

        let result = min_f64(value, value2);
        assert!(result.is_nan());

        let result = max_f64(value, value2);
        assert!(result.is_nan());

        let value = f64::INFINITY;
        assert!(biased_exponent(value) == max_exponent());
    }

    fn test_make_title() {
        let title  = "";
        let name   = "hello";
        let result = make_title(title, name);

        assert!(result == name);

        let title = "say";
        let result = make_title(title, name);

        assert!(result == "say ==> hello");
    }

    fn test_units() {
        let singular = "byte";
        let plural   = "bytes";
        let result   = Units::new(singular, plural);

        assert!(result.singular == "byte" );
        assert!(result.plural   == "bytes");
    }

    fn test_parsing() {
        let printer      = Some(stdout_printer());
        let title        = Some("Title".to_string());
        let merge_min    = 24;
        let merge_max    = 28;
        let no_zero_rows = true;
        let histo_opts   = Some(HistoOpts { merge_min, merge_max, no_zero_rows });
        let units        = bytes();

        let print_opts = Some(PrintOpts { printer, title, histo_opts, units });

        let _     = parse_printer   (&print_opts);
        let title = parse_title     (&print_opts, "default");
        let histo = parse_histo_opts(&print_opts);
        let units = parse_units     (&print_opts);

        assert!( histo.no_zero_rows          );
        assert!( histo.merge_min == merge_min);
        assert!( histo.merge_max == merge_max);
        assert!( units.singular  == "byte"   );
        assert!( units.plural    == "bytes"  );
        assert!( title           == "Title"  );

        let printer    = None;
        let title      = None;
        let histo_opts = None;
        let units      = None;
        let print_opts = Some(PrintOpts { printer, title, histo_opts, units });

        let _     = parse_printer   (&print_opts);
        let title = parse_title     (&print_opts, "default");
        let histo = parse_histo_opts(&print_opts);
        let units = parse_units     (&print_opts);

        assert!(!histo.no_zero_rows          );
        assert!( histo.merge_min == 0        );
        assert!( histo.merge_max == 0        );
        assert!( units.singular  == ""       );
        assert!( units.plural    == ""       );
        assert!( title           == "default");
    }

    fn test_stdio_printer() {
        let mut printer = StdioPrinter::new(StreamKind::Stderr);

        printer.print("test_stdio_printer:  performed output");
    }

    fn test_check_printer_forgive() {
        let expected_output = [ "test_check_printer:  output" ];

        let mut printer = CheckPrinter::new(&expected_output, false);

        printer.print(expected_output[0]);
        printer.print(expected_output[0]);
    }

    #[test]
    #[should_panic]
    fn test_check_printer() {
        let expected_output = [ "test_check_printer:  output" ];

        let mut printer = CheckPrinter::new(&expected_output, true);

        printer.print(expected_output[0]);
        printer.print(expected_output[0]);
    }

    #[test]
    #[should_panic]
    fn test_timer_start() {
        let hz    = 1_000;
        let timer = TestTimer::new_box(hz);

        timer.borrow_mut().start();
    }

    fn test_test_timer_setup () {
        let hz    = 1_000;
        let timer = TestTimer::new_box(hz);

        timer.borrow_mut().setup_elapsed_time(hz as i64);
        timer.borrow_mut().start();

        assert!(timer.borrow_mut().finish() == hz as i64);
    }

    fn test_math() {
        assert!(compute_kurtosis(4,  0.0, 0.0) == 0.0);
        assert!(compute_kurtosis(4,  1.0, 0.0) == 0.0);
        assert!(compute_kurtosis(4, -1.0, 0.0) == 0.0);
        assert!(compute_skewness(4,  0.0, 0.0) == 0.0);
        assert!(compute_skewness(4, -1.0, 0.0) == 0.0);
    }

    #[test]
    pub fn run_lib_tests() {
        test_time_printing();
        test_time_window();
        test_running_time();
        run_all_histo_tests();
        test_test_timer();
        test_test_timer_setup();
        test_float_functions();
        test_make_title();
        test_units();
        test_parsing();
        test_stdio_printer();
        test_check_printer_forgive();
        test_math();
    }
}
