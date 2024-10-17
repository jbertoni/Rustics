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
//!     * For the pseudo-log histogram, the pseudo-log of a negative number n is defines as -log(-n).
//!       The pseudo-log of 0 is defined as 0, and for convenience, the pseudo-log of -(2^64) is defined
//!       as 63.  Logs of positive values are computed by rounding up any fractional part, so the
//!       pseudo-log of 5 is 3.  From the definition, the pseudo-log of -5 is -3.
//!     * The values can be interpreted as time periods with a given hertz.
//!
//! * Integer statistics structs
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
//! * Time statistics structs
//!     * RunningTime
//!         * This type uses the RunningInteger code to handle time intervals.  Values will be
//!           printed using units of time.
//!
//!     * TimeWindow
//!         * This type uses the IntegerWindow code to handle time intervals.  As with the
//!           RunningTime type, values are printed in units of time.
//!
//! * Hierarchial Statistics
//!     * Hierarchical statistics contain sums of integer statistics objects, and can be
//!       multi-level.  Values are recorded into an integer statistic of some kind.  When
//!       the hierarchical struct receives a request to advance to a new statistics object,
//!       the current statistic that is collecting data is pushed into a window, and a new
//!       statistics object will be created to hold any new values to be recorded.
//!
//!     * When a programmable number of statistics objects have been pushed into a window,
//!       these statistics are summed and the sum placed in a high level window.  The
//!       summation is done recursively up to a programmed number of levels.  Each level
//!       has a parameter specifying the number of objects to be summed.
//!
//!     * The lowest level, the one to which statistical data is recorded, can be configured
//!       to push the current object and start a new one after a certain number of samples
//!       have been recorded, or the advance() method can be invoked to move to a new
//!       statistics object.
//!
//!     * Each level of statistics has a programmable "live" count that gives the number of
//!       objects that are summed and pushed to the higher level, and a retention window, so
//!       that statistics are kept for some time period after being summed.
//!
//! * Hierarchical statistics struct
//!     * Hier
//!         * The Hier struct implements the framework for hierarchical statistics.
//!         * The HierGenerator trait provides the interface from a Rustics implementation
//!           to the Hier struct.
//!     * RunningGenerator
//!         * This structure provides RunningInteger types in a Hier structure.  See
//!            RunningGenerator::new_hier() for a simple interface to get going.
//!
//! * Creating Sets
//!     * The "arc_sets" and "rc_sets" modules implement a simple feature allowing the creation of sets
//!       that accept statistics and subsets as members.
//!
//!     * RusticsArcSet
//!         * This type functions as an Arc-based implementation of sets and subsets that can be printed
//!           and cleared on command.
//!
//!     * RusticsRcSet
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

use std::sync::Mutex;
use std::sync::Arc;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::min;
use std::cmp::max;

use time::Timer;

pub mod arc_sets;
pub mod rc_sets;
pub mod hier;
pub mod window;
pub mod time;
pub mod sum;
pub mod running_generator;
pub mod log_histogram;

mod printable;

use hier::Hier;
use hier::HierDescriptor;
use hier::HierConfig;
use hier::HierGenerator;
use hier::HierExporter;
use hier::HierMember;
use hier::ExporterRc;
use hier::MemberRc;
use log_histogram::LogHistogram;
use sum::kbk_sum;
use printable::Printable;

pub type PrinterBox    = Arc<Mutex<dyn Printer>>;
pub type PrinterOption = Option<Arc<Mutex<dyn Printer>>>;
pub type TimerBox      = Rc<RefCell<dyn Timer>>;

pub fn timer_box_hz(timer:  &TimerBox) -> u128 {
    (**timer).borrow().hz()
}

fn stdout_printer() -> PrinterBox {
    Arc::new(Mutex::new(StdioPrinter::new(StreamKind::Stdout)))
}

// Insert commas into a string containing an integer.

pub fn commas(value: &str) -> String {
    if value.len() <= 3 {
        return value.to_string()
    }

    let sign;
    let digits;
    let comma_interval = 3;

    //  A string like "-200" shouldn't be printed as "-,200", so detect and
    //  handle leading signs that'll cause a comma to be added.  If the
    // string length is 1 mod 3 and the top character is a sign, we need to
    // intervene.

    if value.len() % comma_interval == 1 {
        match value.chars().next().unwrap() {
            '+' => { sign = "+"; digits = value[1..].to_string(); }
            '-' => { sign = "-"; digits = value[1..].to_string(); }
            _   => { sign = ""; digits = value.to_string(); }
        }
    } else {
        sign   = "";
        digits = value.to_string()
    }

    let result =
        digits
            .as_bytes()                 // convert the input to a byte array
            .rchunks(comma_interval)    // break into chunks of three (or whatever) from the right
            .rev()                      // reverse the current order back to the original order
            .map(std::str::from_utf8)   // convert back to a vector of strings
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",");                 // join the blocks of three digits with commas

    let result =
        match sign {
            "+" => "+".to_string() + &result,
            "-" => "-".to_string() + &result,
            _   => result,
        };

    result
}

// Convert an i64 into a string with comma separators.

pub fn commas_i64(value: i64) -> String {
    let base = value.to_string();
    commas(&base)
}

// Convert an u64 into a string with comma separators.

pub fn commas_u64(value: u64) -> String {
    let base = value.to_string();
    commas(&base)
}

pub fn compute_variance(count: u64, moment_2: f64) -> f64 {
    if count < 2 {
        return 0.0;
    }

    let n = count as f64;
    let sample_variance = moment_2 / (n - 1.0);

    sample_variance
}

// Compute the sample skewness.
//
// This formula is from brownmath.com.

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
    let sample_skewness = correction * skewness;

    sample_skewness
}

// Compute the sample kurtosis.
//
// This formula is from brownmath.com

pub fn compute_kurtosis(count: u64, moment_2: f64, moment_4: f64) -> f64 {
    if count < 4 || moment_2 == 0.0 {
        return 0.0;
    }

    assert!(moment_2 > 0.0 && moment_4 >= 0.0);

    let n               = count as f64;
    let kurtosis        = moment_4 / (moment_2.powf(2.0) / n) - 3.0;
    let correction      = (n - 1.0) / ((n - 2.0) * (n - 3.0));
    let kurtosis_factor = (n + 1.0) * kurtosis + 6.0;

    let sample_excess_kurtosis = correction * kurtosis_factor;

    sample_excess_kurtosis
}

// Insert a delimiter and concatenate the parent and child names
// when creating a hierarchical title.

pub fn create_title(title_prefix: &str, title: &str) -> String {
    let title =
        if title_prefix.is_empty() {
            title.to_string()
        } else {
            let mut full_title = String::from(title_prefix);
            full_title.push_str(" ==> ");
            full_title.push_str(title);
            full_title
        };

    title
}

// Define a Printer trait to allow a custom stream for print() operations.
//
// This routine is invoked for each line to be printed.  The print() member
// is responsible for adding the newline, either via println!() or some
// other mechanism.

pub trait Printer {
    fn print(&self, output: &str);
}

// Define a printer that will send output to Stdout or Stderr, as
// configured.

pub struct StdioPrinter {
    which: StreamKind,
}

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

// Define the main trait for collecting statistics.  Eventually, this
// library will support floating point samples.

pub trait Rustics {
    fn record_i64(&mut self, sample: i64);   // add an i64 sample
    fn record_f64(&mut self, sample: f64);   // add an f64 sample -- not implemented
    fn record_event(&mut self);              // implementation-specific record
    fn record_time(&mut self, sample: i64);  // add a time sample

    fn record_interval(&mut self, timer: &mut TimerBox);
                                             // Add a duration sample ending now

    fn name(&self)               -> String;  // a text (UTF-8) name to print
    fn title(&self)              -> String;  // a text (UTF-8) name to print
    fn class(&self)              -> &str;    // the type of a sample:  integer or floating
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

    fn print(&self);
    fn print_opts(&self, printer: PrinterOption, title: Option<&str>);
    fn set_title(&mut self, title: &str);

    // For internal use only.
    fn set_id(&mut self, index: usize);
    fn id(&self)                          -> usize;
    fn equals(&self, other: &dyn Rustics) -> bool;
    fn generic(&self)                     -> &dyn Any;
    fn histo_log_mode(&self)              -> i64;
}

pub trait Histogram {
    fn log_histogram(&self) -> LogHistogram;
    fn print_histogram(&self);
}

// Define the implementation of a very simple running integer sample space.

#[derive(Clone)]
pub struct RunningInteger {
    name:       String,
    title:      String,
    id:         usize,

    count:      u64,
    mean:       f64,
    moment_2:   f64,
    moment_3:   f64,
    moment_4:   f64,

    min:        i64,
    max:        i64,

    pub log_histogram:    LogHistogram,

    printer:    PrinterBox,
}

// RunningExporter structs are used to export statistics from a RunningInteger
// struct so that multiple structures can be summed.

pub struct RunningExporter {
    addends: Vec<RunningExport>,
}

impl RunningExporter {
    fn new() -> RunningExporter {
        let addends = Vec::new();

        RunningExporter { addends }
    }

    fn push(&mut self, addend: RunningExport) {
        self.addends.push(addend);
    }

    // Make a member based on the summed exports.

    fn make_member(&mut self, name: &str, printer: PrinterBox) -> RunningInteger {
        let title   = name;
        let sum     = sum_running(&self.addends);
        let printer = Some(printer);

        RunningInteger::new_from_exporter(name, title, printer, sum)
    }
}

// The Hier code uses this trait to do summation of statistics.
//
// We just need downcasting capabilities since all the work
// is implementation-specific.

impl HierExporter for RunningExporter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct RunningExport {
    pub count:      u64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,

    pub min:        i64,
    pub max:        i64,

    pub log_histogram:    LogHistogram,
}

pub fn sum_log_histogram(sum:  &mut LogHistogram, addend: &LogHistogram) {
    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }
}

// Merge the vector of exported statistics.  Many fields are just
// sums of the parts.

pub fn sum_running(exports: &Vec::<RunningExport>) -> RunningExport {
    let mut count          = 0;
    let mut min            = i64::MAX;
    let mut max            = i64::MIN;
    let mut log_histogram  = LogHistogram::new();

    let mut mean_vec       = Vec::with_capacity(exports.len());
    let mut moment_2_vec   = Vec::with_capacity(exports.len());
    let mut moment_3_vec   = Vec::with_capacity(exports.len());
    let mut moment_4_vec   = Vec::with_capacity(exports.len());

    for export in exports {
        count    += export.count;
        min       = std::cmp::min(min, export.min);
        max       = std::cmp::max(max, export.max);

        sum_log_histogram(&mut log_histogram, &export.log_histogram);

        mean_vec.push(export.mean * export.count as f64);
        moment_2_vec.push(export.moment_2);
        moment_3_vec.push(export.moment_3);
        moment_4_vec.push(export.moment_4);
    }

    let mean     = kbk_sum(&mut mean_vec[..]) / count as f64;
    let moment_2 = kbk_sum(&mut moment_2_vec[..]);
    let moment_3 = kbk_sum(&mut moment_3_vec[..]);
    let moment_4 = kbk_sum(&mut moment_4_vec[..]);

    RunningExport { count, mean, moment_2, moment_3, moment_4, min, max, log_histogram }
}

impl RunningInteger {
    pub fn new(name_in: &str, printer: PrinterOption) -> RunningInteger {
        let name            = String::from(name_in);
        let title           = String::from(name_in);
        let id              = usize::MAX;
        let count           = 0;
        let mean            = 0.0;
        let moment_2        = 0.0;
        let moment_3        = 0.0;
        let moment_4        = 0.0;
        let min             = i64::MAX;
        let max             = i64::MIN;
        let log_histogram   = LogHistogram::new();

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer
        }
    }

    pub fn new_from_exporter(name: &str, title: &str, printer: PrinterOption, import: RunningExport)
            -> RunningInteger {
        let name            = String::from(name);
        let title           = String::from(title);
        let id              = usize::MAX;
        let count           = import.count;
        let mean            = import.mean;
        let moment_2        = import.moment_2;
        let moment_3        = import.moment_3;
        let moment_4        = import.moment_4;
        let min             = import.min;
        let max             = import.max;
        let log_histogram   = import.log_histogram;

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer
        }
    }

    // Export all the statistics from a given structure to
    // be used to create a sum of many structures.

    pub fn export(&self) -> RunningExport {
        let count           = self.count;
        let mean            = self.mean;
        let moment_2        = self.moment_2;
        let moment_3        = self.moment_3;
        let moment_4        = self.moment_4;
        let log_histogram   = self.log_histogram.clone();
        let min             = self.min;
        let max             = self.max;

        RunningExport {
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max
        }
    }
}

// The formula for computing the second moment for the variance (moment_2)
// is from D. E. Knuth, The Art of Computer Programming.

impl Rustics for RunningInteger {
    fn record_i64(&mut self, sample: i64) {
        self.count += 1;

        self.log_histogram.record(sample);

        let sample_f64 = sample as f64;

        if self.count == 1 {
            self.mean     = sample_f64;
            self.moment_2 = 0.0;
            self.moment_3 = 0.0;
            self.moment_4 = 0.0;
            self.min      = sample;
            self.max      = sample;
        } else {
            let distance_mean     = sample_f64 - self.mean;
            let new_mean          = self.mean + distance_mean / self.count as f64;
            let distance_new_mean = sample_f64 - new_mean;
            let square_estimate   = distance_mean * distance_new_mean;
            let cube_estimate     = square_estimate * square_estimate.sqrt();
            let new_moment_2      = self.moment_2 + square_estimate;
            let new_moment_3      = self.moment_3 + cube_estimate;
            let new_moment_4      = self.moment_4 + square_estimate * square_estimate;

            self.mean             = new_mean;
            self.moment_2         = new_moment_2;
            self.moment_3         = new_moment_3;
            self.moment_4         = new_moment_4;
            self.min              = min(self.min, sample);
            self.max              = max(self.max, sample);
        }
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::RunningInteger:  f64 samples are not permitted.");
    }

    fn record_event(&mut self) {
        panic!("Rustics::RunningInteger:  event samples are not permitted.");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Rustics::RunningInteger:  time samples are not permitted.");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Rustics::RunningInteger:  time intervals are not permitted.");
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        "integer"
    }

    fn count(&self) -> u64 {
        self.count
    }

    fn log_mode(&self) -> isize {
        self.log_histogram.log_mode()
    }

    fn mean(&self) -> f64 {
        self.mean
    }

    fn standard_deviation(&self) -> f64 {
        self.variance().sqrt()
    }

    fn variance(&self) -> f64 {
        compute_variance(self.count, self.moment_2)
    }

    fn skewness(&self) -> f64 {
        compute_skewness(self.count, self.moment_2, self.moment_3)
    }

    fn kurtosis(&self) -> f64 {
        compute_kurtosis(self.count, self.moment_2, self.moment_4)
    }

    fn precompute(&mut self) {
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn min_i64(&self) -> i64 {
        self.min
    }

    fn max_i64(&self) -> i64 {
        self.max
    }

    fn min_f64(&self) -> f64 {
        self.min as f64
    }

    fn max_f64(&self) -> f64 {
        self.max as f64
    }

    fn clear(&mut self) {
        self.count    = 0;
        self.mean     = 0.0;
        self.moment_2 = 0.0;
        self.moment_3 = 0.0;
        self.moment_4 = 0.0;
        self.min      = i64::MIN;
        self.max      = i64::MAX;

        self.log_histogram.clear();
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<RunningInteger>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        self.log_histogram.log_mode() as i64
    }

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

        let n        = self.count;
        let min      = self.min;
        let max      = self.max;
        let log_mode = self.log_histogram.log_mode() as i64;
        let mean     = self.mean;
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        println!("print_opts:  getting printer lock");
        let printer  = &mut *printer_box.lock().unwrap();
        println!("print_opts:  got printer lock");

        printer.print(title);
        printable.print_common_integer(printer);
        printable.print_common_float(printer);
        self.log_histogram.print(printer);
    }

    fn set_title(&mut self, title: &str) {
        self.title = String::from(title)
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }
}

impl Histogram for RunningInteger {
    fn log_histogram(&self) -> LogHistogram {
        self.log_histogram.clone()
    }

    fn print_histogram(&self) {
        let printer = &mut *self.printer.lock().unwrap();
        self.log_histogram.print(printer);
    }
}

// Define the implementation of a very simple running integer window sample.

pub struct IntegerWindow {
    name:           String,
    title:          String,
    window_size:    usize,
    vector:         Vec<i64>,
    id:             usize,

    //  These fields must be zeroed or reset in clear():

    index:          usize,
    stats_valid:    bool,

    //  These fields are computed when stats_valid is false

    mean:           f64,
    sum:            f64,
    moment_2:       f64,
    moment_3:       f64,
    moment_4:       f64,

    pub log_histogram:  LogHistogram,

    printer:        PrinterBox,
}

// The Crunched structure contains all the data needed to
// compute the summary statistics that we need to print.

struct Crunched {
    mean:       f64,
    sum:        f64,
    moment_2:   f64,
    moment_3:   f64,
    moment_4:   f64,
}

impl Crunched {
    pub fn new() -> Crunched {
        let mean     = 0.0;
        let sum      = 0.0;
        let moment_2 = 0.0;
        let moment_3 = 0.0;
        let moment_4 = 0.0;

        Crunched { mean, sum, moment_2, moment_3, moment_4 }
    }
}

impl IntegerWindow {
    pub fn new(name_in: &str, window_size: usize) -> IntegerWindow {
        if window_size == 0 {
            panic!("The window size is zero.");
        }

        let name          = String::from(name_in);
        let title         = String::from(name_in);
        let id            = usize::MAX;
        let vector        = Vec::with_capacity(window_size);
        let index         = 0;
        let stats_valid   = false;
        let mean          = 0.0;
        let sum           = 0.0;
        let moment_2      = 0.0;
        let moment_3      = 0.0;
        let moment_4      = 0.0;
        let log_histogram = LogHistogram::new();
        let printer       = stdout_printer();

        IntegerWindow {
            name,
            title,
            id,
            window_size,
            vector,
            index,
            stats_valid,
            mean,
            sum,
            moment_2,
            moment_3,
            moment_4,
            log_histogram,
            printer
        }
    }

    fn sum(&self) -> f64 {
        let mut sum = 0.0;

        for sample in self.vector.iter() {
            sum += *sample as f64;
        }

        sum
    }

    fn crunch(&self) -> Crunched {
        if self.vector.is_empty() {
            return Crunched::new();
        }

        let mut sum = 0.0;

        for sample in self.vector.iter() {
            sum += *sample as f64;
        }

        let mean = sum / self.vector.len() as f64;
        let mut moment_2 = 0.0;
        let mut moment_3 = 0.0;
        let mut moment_4 = 0.0;

        for sample in self.vector.iter() {
            let distance = *sample as f64 - mean;
            let square   = distance * distance;

            moment_2 += square;
            moment_3 += distance * square;
            moment_4 += square * square;
        }

        Crunched { mean, sum, moment_2, moment_3, moment_4 }
    }

    fn compute_min(&self) -> i64 {
        match self.vector.iter().min() {
            Some(min) => *min,
            None => 0,
        }
    }

    fn compute_max(&self) -> i64 {
        match self.vector.iter().max() {
            Some(max) => *max,
            None => 0,
        }
    }
}

impl Rustics for IntegerWindow {
    fn record_i64(&mut self, sample: i64) {
        if self.vector.len() == self.window_size {
            self.vector[self.index] = sample;
            self.index += 1;

            if self.index >= self.window_size {
                self.index = 0;
            }
        } else {
            self.vector.push(sample);
        }

        self.log_histogram.record(sample);
        self.stats_valid = false;
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::IntegerWindow:  f64 samples are not permitted.");
    }

    fn record_event(&mut self) {
        panic!("Rustics::IntegerWindow:  event samples are not permitted.");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Rustics::IntegerWindow:  time samples are not permitted.");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Rustics::IntegerWindow:  time intervals are not permitted.");
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        "integer"
    }

    fn count(&self) -> u64 {
        self.vector.len() as u64
    }

    fn log_mode(&self) -> isize {
        self.log_histogram.log_mode()
    }

    fn mean(&self) -> f64 {
        if self.vector.is_empty() {
            return 0.0;
        }

        if self.stats_valid {
            return self.mean;
        }

        let sample_sum = self.sum();
        sample_sum / self.vector.len() as f64
    }

    fn standard_deviation(&self) -> f64 {
        self.variance().sqrt()
    }

    fn variance(&self) -> f64 {
        let count = self.vector.len() as u64;

        let variance =
            if self.stats_valid {
                compute_variance(count, self.moment_2)
            } else {
                let crunched = self.crunch();
                compute_variance(count, crunched.moment_2)
            };

        variance
    }

    fn skewness(&self) -> f64 {
        let count = self.vector.len() as u64;

        compute_skewness(count, self.moment_2, self.moment_3)
    }

    fn kurtosis(&self) -> f64 {
        let count = self.vector.len() as u64;

        compute_kurtosis(count, self.moment_2, self.moment_4)
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn min_f64(&self) -> f64 {
        self.compute_min() as f64
    }

    fn max_f64(&self) -> f64 {
        self.compute_max() as f64
    }

    fn min_i64(&self) -> i64 {
        self.compute_min()
    }

    fn max_i64(&self) -> i64 {
        self.compute_max()
    }

    fn precompute(&mut self) {
        if self.stats_valid {
            return;
        }

        let crunched = self.crunch();

        self.mean        = crunched.mean;
        self.sum         = crunched.sum;
        self.moment_2    = crunched.moment_2;
        self.moment_3    = crunched.moment_3;
        self.moment_4    = crunched.moment_4;
        self.stats_valid = true;
    }

    fn clear(&mut self) {
        self.vector.clear();
        self.index = 0;
        self.log_histogram.clear();

        self.stats_valid = false;
    }

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

        let printer = &mut *printer_box.lock().unwrap();

        let n        = self.vector.len() as u64;
        let min      = self.compute_min();
        let max      = self.compute_max();
        let log_mode = self.log_histogram.log_mode() as i64;

        let mean;
        let variance;
        let skewness;
        let kurtosis;

        if self.stats_valid {
            mean     = self.mean();
            variance = self.variance();
            skewness = self.skewness();
            kurtosis = self.kurtosis();
        } else {
            let crunched = self.crunch();

            mean     = crunched.mean;
            variance = compute_variance(n, crunched.moment_2);
            skewness = compute_skewness(n, crunched.moment_2, crunched.moment_3);
            kurtosis = compute_kurtosis(n, crunched.moment_2, crunched.moment_4);
        }

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(title);
        printable.print_common_integer(printer);
        printable.print_common_float(printer);
        self.log_histogram.print(printer);
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<IntegerWindow>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        self.log_histogram.log_mode() as i64
    }

    fn set_title(&mut self, title: &str) {
        self.title = String::from(title)
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }
}

impl Histogram for IntegerWindow {
    fn log_histogram(&self) -> LogHistogram {
        self.log_histogram.clone()
    }

    fn print_histogram(&self) {
        let printer = &mut *self.printer.lock().unwrap();
        self.log_histogram.print(printer);
    }
}

#[derive(Clone)]
pub struct RunningTime {
    printer:            PrinterBox,

    running_integer:    Box<RunningInteger>,
    timer:              TimerBox,
    hz:                 i64,
}

impl RunningTime {
    pub fn new(name_in: &str, timer: TimerBox) -> RunningTime {
        let id      = usize::MAX;
        let name    = String::from(name_in);
        let title   = String::from(name_in);
        let printer = stdout_printer();
        let hz      = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::RunningTime:  The timer hz value is too large.");
        }

        let hz = hz as i64;

        let running_integer =
            Box::new(RunningInteger {
                name,
                title,
                id,
                count:           0,
                mean:          0.0,
                moment_2:      0.0,
                moment_3:      0.0,
                moment_4:      0.0,
                log_histogram: LogHistogram::new(),
                min:           i64::MAX,
                max:           i64::MIN,
                printer
            });

        let printer = stdout_printer();

        RunningTime { printer, running_integer, timer, hz }
    }

    pub fn hz(&self) -> i64 {
        self.hz
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
        let mut timer    = (*self.timer).borrow_mut();
        let     interval = timer.finish();  // read and restart the timer

        if interval > i64::MAX as u128 {
            panic!("RunningTime::record_interval:  The interval is too long.");
        }

        self.running_integer.record_i64(interval as i64);
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.running_integer.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
        let interval = timer.finish();

        if interval > i64::MAX as u128 {
            panic!("RunningTime::record_interval:  The interval is too long.");
        }

        self.running_integer.record_i64(interval as i64);
    }

    fn name(&self) -> String {
        self.running_integer.name()
    }

    fn title(&self) -> String {
        self.running_integer.title()
    }

    fn class(&self) -> &str {
        self.running_integer.class()
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
                printer.clone()
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.running_integer.title()
            };

        let printer  = &mut *printer_box.lock().unwrap();
        let n        = self.count();
        let min      = self.min_i64();
        let max      = self.max_i64();
        let log_mode = self.running_integer.histo_log_mode();
        let mean     = self.mean();
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);

        self.running_integer.print_histogram();
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.running_integer.set_title(title);
    }

    fn set_id(&mut self, index: usize) {
        self.running_integer.set_id(index)
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

    fn histo_log_mode(&self) -> i64 {
        self.running_integer.histo_log_mode()
    }
}

pub struct TimeWindow {
    printer:            PrinterBox,

    integer_window:     Box<IntegerWindow>,
    timer:              TimerBox,
    hz:                 i64,
}

impl TimeWindow {
    pub fn new(name_in: &str, window_size: usize, timer:  TimerBox) -> TimeWindow {
        let id            = usize::MAX;
        let name          = String::from(name_in);
        let title         = String::from(name_in);
        let printer       = stdout_printer();
        let hz            = timer_box_hz(&timer);
        let log_histogram = LogHistogram::new();
        let stats_valid   = false;

        if hz > i64::MAX as u128 {
            panic!("Rustics::TimeWindow:  The timer hz value is too large.");
        }

        let hz = hz as i64;
        let vector = Vec::with_capacity(window_size);

        let integer_window =
            Box::new(IntegerWindow {
                name,
                title,
                index: 0,
                window_size,
                vector,
                id,
                stats_valid,
                mean: 0.0,
                sum: 0.0,
                moment_2: 0.0,
                moment_3: 0.0,
                moment_4: 0.0,
                log_histogram,
                printer
            });

        let printer  = stdout_printer();

        TimeWindow { printer, integer_window, timer, hz }
    }

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

    fn record_event(&mut self) {
        let interval = (*self.timer).borrow_mut().finish();

        if interval > i64::MAX as u128 {
            panic!("TimeWindow::record_interval:  The interval is too long.");
        }

        self.integer_window.record_i64(interval as i64);
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.integer_window.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
        let interval = timer.finish();

        if interval > i64::MAX as u128 {
            panic!("TimeWindow::record_interval:  The interval is too long.");
        }

        self.integer_window.record_i64(interval as i64);
    }

    fn name(&self) -> String {
        self.integer_window.name()
    }

    fn title(&self) -> String {
        self.integer_window.title()
    }

    fn class(&self) -> &str {
        self.integer_window.class()
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
                printer.clone()
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.integer_window.title()
            };

        let crunched  = self.integer_window.crunch();

        let n         = self.integer_window.count();
        let min       = self.integer_window.min_i64();
        let max       = self.integer_window.max_i64();
        let log_mode  = self.integer_window.histo_log_mode();

        let mean      = crunched.mean;
        let variance  = compute_variance(n, crunched.moment_2);
        let skewness  = compute_skewness(n, crunched.moment_2, crunched.moment_3);
        let kurtosis  = compute_kurtosis(n, crunched.moment_2, crunched.moment_4);

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        let printer   = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);

        self.integer_window.print_histogram();
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.integer_window.set_title(title)
    }

    fn set_id(&mut self, index: usize) {
        self.integer_window.set_id(index)
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

    fn histo_log_mode(&self) -> i64 {
        self.integer_window.histo_log_mode()
    }
}

pub struct Counter {
    name:       String,
    title:      String,
    count:      i64,
    id:         usize,
    printer:    PrinterBox,
}

impl Counter {
    pub fn new(name: &str) -> Counter {
        let name    = String::from(name);
        let title   = name.clone();
        let count   = 0;
        let id      = 0;
        let printer = stdout_printer();

        Counter { name, title, count, id, printer }
    }
}

impl Rustics for Counter {
    fn record_i64(&mut self, sample: i64) {
        if sample < 0 {
            panic!("Counter::record_i64:  The sample is negative.");
        }

        self.count += sample;
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Counter::record_f64:  not supported");
    }

    fn record_event(&mut self) {
        self.count += 1;
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Counter::record_time:  not supported");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Counter::record_interval:  not supported");
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        "integer"
    }

    fn count(&self) -> u64 {
        self.count as u64
    }

    fn log_mode(&self) -> isize {
        panic!("Counter::log_mode:  not supported");
    }

    fn mean(&self) -> f64 {
        panic!("Counter::mean:  not supported");
    }

    fn standard_deviation(&self) -> f64 {
        panic!("Counter::standard_deviation:  not supported");
    }

    fn variance(&self) -> f64 {
        panic!("Counter::variance:  not supported");
    }

    fn skewness(&self) -> f64 {
        panic!("Counter::skewness:  not supported");
    }

    fn kurtosis(&self) -> f64 {
        panic!("Counter::kurtosis:  not supported");
    }

    fn int_extremes(&self) -> bool {
        false
    }

    fn min_i64(&self) -> i64 {
        panic!("Counter::min_i64:  not supported");
    }

    fn min_f64(&self) -> f64 {
        panic!("Counter::min_f64:  not supported");
    }

    fn max_i64(&self) -> i64 {
        panic!("Counter::max_i64:  not supported");
    }

    fn max_f64(&self) -> f64 {
        panic!("Counter::max_f64:  not supported");
    }

    fn precompute(&mut self) {
    }

    fn clear(&mut self) {
        self.count = 0;
    }

    // Functions for printing:

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

        let printer = &mut *printer_box.lock().unwrap();

        printer.print(title);
        Printable::print_integer("Count", self.count, printer);
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.title = String::from(title);
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<Counter>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        panic!("Counter::histo_log_mode:  not supported");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use crate::hier::*;
    use crate::log_histogram::pseudo_log_index;
    use crate::running_generator::RunningGenerator;

    pub fn test_commas() {
        let test = [ 123456, 12, -1, -1234, 4000000, -200, -2000, -20000 ];
        let expect = [ "123,456", "12", "-1", "-1,234", "4,000,000", "-200", "-2,000", "-20,000" ];
        let mut i = 0;

        for sample in test.iter() {
            println!("Test:  {} vs {}", commas_i64(*sample), expect[i]);
            assert_eq!(commas_i64(*sample), expect[i]);
            i += 1;
        }

        assert_eq!(commas("+21"), "+21");
        assert_eq!(commas("+212"), "+212");
        assert_eq!(commas("+2123"), "+2,123");
        assert_eq!(commas("+21234"), "+21,234");
        assert_eq!(commas("+212345"), "+212,345");

        assert_eq!(commas("+20"), "+20");
        assert_eq!(commas("+200"), "+200");
        assert_eq!(commas("+2000"), "+2,000");
        assert_eq!(commas("+20000"), "+20,000");
        assert_eq!(commas("+200000"), "+200,000");
    }

    pub fn test_log_histogram() {
        let mut histogram = LogHistogram::new();
        let     printer   = &mut TestPrinter { test_output: &"Test Output" };
        let     test      = [ 1, -1, 4, 25, 4109, -4108, -8, -9, -16, -17, 3, 8, 16 ];

        for i in test.iter() {
            histogram.record(*i);
        }

        histogram.print(printer);
    }

    pub fn test_pseudo_log() {
        let test   = [ 1, 0, -1, -4, -3, i64::MIN, 3, 4, 5, 8, i64::MAX ];
        let expect = [ 0, 0,  0,  2,  2,       63, 2, 2, 3, 3,       63 ];

        let mut i = 0;

        for sample in test.iter() {
            println!("pseudo_log_index({}) = {}", *sample, pseudo_log_index(*sample));
            assert_eq!(pseudo_log_index(*sample), expect[i]);
            i += 1;
        }
    }

    struct TestPrinter {
        test_output: &'static str,
    }

    impl Printer for TestPrinter {
        fn print(&self, output: &str) {
            println!("{}:  {}", self.test_output, output);
        }
    }

    pub fn test_simple_running_integer() {
        let mut stats = RunningInteger::new(&"Test Statistics", None);

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        let printer = Arc::new(Mutex::new(TestPrinter { test_output: "test header ======" } ));
        stats.print_opts(Some(printer), None);
    }

    pub fn test_simple_integer_window() {
        let window_size = 100;
        let mut stats = IntegerWindow::new(&"Test Statistics", window_size);

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        assert!(stats.log_mode() as usize == pseudo_log_index(stats.max_i64()));
        stats.print();
        let sample = 100;

        for _i in 0..2 * window_size {
            stats.record_i64(sample);
        }

        stats.print();
        assert!(stats.mean() == sample as f64);
        assert!(stats.log_mode() as usize == pseudo_log_index(sample));
    }

    static global_next: Mutex<u128> = Mutex::new(0 as u128);

    fn get_global_next() -> u128 {
        *(global_next.lock().unwrap())
    }

    fn set_global_next(value: u128) {
        *(global_next.lock().unwrap()) = value;
    }

    struct TestTimer {
        start: u128,
        hz: u128,
    }

    impl TestTimer {
        fn new(hz: u128) -> TestTimer {
            let start = 0;

            TestTimer { start, hz }
        }
    }

    impl Timer for TestTimer {
        fn start(&mut self) {
            assert!(get_global_next() > 0);
            self.start = get_global_next();
        }

        fn finish(&mut self) -> u128 {
            assert!(self.start > 0);
            assert!(get_global_next() >= self.start);
            let elapsed_time = get_global_next() - self.start;
            self.start = 0;
            set_global_next(0);
            elapsed_time
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    fn setup_elapsed_time(timer: &mut TimerBox, ticks: i64) {
        assert!(ticks >= 0);
        let mut timer = (**timer).borrow_mut();
        set_global_next(1);
        timer.start();
        set_global_next(ticks as u128 + 1);
    }

    fn test_simple_running_time() {
        println!("Testing running time statistics.");

        let     hz              = 1_000_000_000;
        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(hz)));
        let mut time_stat       = RunningTime::new("Test Running Time 1", timer.clone());

        setup_elapsed_time(&mut timer, i64::MAX);
        time_stat.record_event();

        assert!(time_stat.min_i64() == i64::MAX);
        assert!(time_stat.max_i64() == i64::MAX);

        setup_elapsed_time(&mut timer, 0);
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

            setup_elapsed_time(&mut timer, interval);
            time_stat.record_event();
        }

        time_stat.print();

        // Okay, use a more restricted range of times.

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat = RunningTime::new("Test Running Time 2", timer.clone());

        let limit = 99;

        for i in 0..limit + 1 {
            let interval = i * i * i;
            setup_elapsed_time(&mut timer, interval);

            if i & 1 != 0 {
                time_stat.record_event();
            } else {
                time_stat.record_interval(&mut timer);
            }
        }

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == limit * limit * limit);

        time_stat.print();

        // Get a sample with easily calculated summary statistics

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat = RunningTime::new("Test Running Time => 1..100", timer.clone());

        for i in 1..101 {
            setup_elapsed_time(&mut timer, i);
            time_stat.record_event();
        }

        time_stat.print();

        // Cover all the scales.

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Running Time => Scale Test", timer.clone());

        let mut time    = 1;
        let     printer = &mut StdioPrinter::new(StreamKind::Stdout);

        for i in 1..16 {
            setup_elapsed_time(&mut timer, time);

            if i & 1 != 0 {
                time_stat.record_event();
            } else {
                time_stat.record_interval(&mut timer);
            }


            let header = format!("{} => ", commas_i64(time));
            Printable::print_time(&header, time as f64, hz as i64, printer);

            time *= 10;
        }

        time_stat.print();
    }

    fn test_simple_time_window() {
        println!("Testing time windows.");

        let     hz              = 1_000_000_000;
        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(hz)));
        let mut time_stat       = TimeWindow::new("Test Time Window 1", 50, timer.clone());

        setup_elapsed_time(&mut timer, i64::MAX);
        time_stat.record_event();

        assert!(time_stat.min_i64() == i64::MAX);
        assert!(time_stat.max_i64() == i64::MAX);

        setup_elapsed_time(&mut timer, 0);
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

            setup_elapsed_time(&mut timer, interval);
            time_stat.record_event();
        }

        time_stat.print();

        // Okay, use a more restricted range of times.

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Time Window 2", timer.clone());

        let limit = 99;

        for i in 0..limit + 1 {
            let interval = i * i * i;
            setup_elapsed_time(&mut timer, interval);
            time_stat.record_event();
        }

        assert!(time_stat.min_i64() == 0);
        assert!(time_stat.max_i64() == limit * limit * limit);

        time_stat.print();

        // Get a sample with easily calculated summary statistics

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Time Window => 1..100", timer.clone());

        for i in 1..101 {
            setup_elapsed_time(&mut timer, i);
            time_stat.record_event();
        }

        time_stat.print();

        // Cover all the scales.

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Time => Scale Test", timer.clone());

        let mut time    = 1;
        let     printer = &mut StdioPrinter::new(StreamKind::Stdout);

        for _i in 1..16 {
            setup_elapsed_time(&mut timer, time);
            time_stat.record_event();
            let header = format!("{} => ", commas_i64(time));
            Printable::print_time(&header, time as f64, hz as i64, printer);

            time *= 10;
        }

        time_stat.print();
    }

    fn test_simple_count() {
        let test_limit  = 20;
        let mut counter = Counter::new("test counter");

        for i in 1..test_limit + 1 {
            counter.record_event();
            counter.record_i64(i);
        }

        let events   = test_limit;
        let sequence = ((test_limit + 1) * test_limit) / 2;
        let expected = events + sequence;

        assert!(counter.count == expected);

        counter.print();
    }

    fn make_hier_gen(generator:  GeneratorRc) -> Hier {
        let     auto_next      = 4;
        let     levels         = 4;
        let     level_0_period = 8;
        let     dimension      = HierDimension::new(level_0_period, 3 * level_0_period);
        let mut dimensions     = Vec::<HierDimension>::with_capacity(levels);
        let     class          = "integer".to_string();

        // Push the level 0 descriptor.

        dimensions.push(dimension);

        // Create a hierarchy.

        let mut period = 4;

        for _i in 1..levels {
            let dimension = HierDimension::new(period, 3 * period);

            dimensions.push(dimension);

            period += 2;
        }

        let descriptor    = HierDescriptor::new(dimensions, Some(auto_next));
        let name          = "test hier".to_string();
        let title         = "test hier".to_string();
        let printer       = stdout_printer();

        let configuration = HierConfig { descriptor, generator, class, name, title, printer };

        Hier::new(configuration)
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     generator    = RunningGenerator::new();
        let     printer      = stdout_printer();
        let     member_rc    = generator.make_member("test member", printer);
        let     member_clone = member_rc.clone();
        let mut member       = member_clone.borrow_mut();
        let     value        = 42;

        member.to_rustics_mut().record_i64(value);

        assert!(member.to_rustics().count() == 1);
        assert!(member.to_rustics().mean()  == value as f64);

        // Drop the lock on the member.

        drop(member);

        // Now try try making an exporter and check basic sanity of as_any_mut.

        let exporter_rc     = generator.make_exporter();
        let exporter_clone  = exporter_rc.clone();

        // Push the member's numbers onto the exporter.

        generator.push(exporter_clone, member_rc);

        let new_member_rc = generator.make_from_exporter("member export", stdout_printer(), exporter_rc);


        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);

        // Now make an actual hier struct.

        let     generator     = Rc::from(RefCell::new(generator));
        let mut hier          = make_hier_gen(generator);

        let mut events = 0;

        for i in 0..100 {
            hier.record_i64(i);

            events += 1;
        }

        assert!(hier.event_count() == events);
        hier.print();
    }

    #[test]
    pub fn run_tests() {
        test_running_generator();
        test_commas();
        test_log_histogram();
        test_log_histogram();
        test_pseudo_log();
        test_simple_running_integer();
        test_simple_integer_window();
        test_simple_time_window();
        test_simple_running_time();
        test_simple_count();
    }
}
