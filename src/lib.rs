//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where 
//  permitted by law.
//

//! 'Rustics' provides a very simple interface for recording events and printing statistics.
//!
//! ## Types
//!
//! * Recording Integers
//!
//!     * Integer statistics provide basic parameters, like the mean, and a pseudo-log histogram.
//!     * For the pseudo-log histogram, the pseudo-log of a negative number n is defines as -log(-n).
//!       The pseudo-log of 0 is defined as 0, and for convenience, the pseudo-log of -(2^64) is defined
//!       as 63.  Logs are computed by rounding up any fractional part, so the pseudo-log of 5 is 3.
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
//! * Creating Sets
//!
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

use time::Timer;

pub mod arc_sets;
pub mod rc_sets;
pub mod time;
pub mod sum;

pub type PrinterBox = Arc<Mutex<dyn Printer>>;
pub type TimerBox = Rc<RefCell<dyn Timer>>;

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

    /*
     *  A string like "-200" shouldn't be printed as "-,200", so detect and
     *  handle leading signs that'll cause a comma to be added.  If the
     * string length is 1 mod 3 and the top character is a sign, we need to
     * intervene.
     */
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

fn print_common_integer_times(data: &Printable, hz: i64, printer: &mut dyn Printer) {
    print_integer("Count", data.n as i64, printer);

    if data.n > 0 {
        print_time("Minumum",  data.min as f64,      hz, printer);
        print_time("Maximum",  data.max as f64,      hz, printer);
        print_time("Log Mode", data.log_mode as f64, hz, printer);
    }
}

fn print_common_float_times(data: &Printable, hz: i64, printer: &mut dyn Printer) {
    if data.n > 0 {
        print_time("Mean",     data.mean,            hz, printer);
        print_time("Std Dev",  data.variance.sqrt(), hz, printer);
        print_time("Variance", data.variance,        hz, printer);
        print_time("Skewness", data.skewness,        hz, printer);
        print_time("Kurtosis", data.kurtosis,        hz, printer);
    }
}

// Format a time value for printing.

//  days
//  hours
//  minutes
//  seconds

pub fn print_time(name: &str, time: f64, hz: i64, printer: &mut dyn Printer) {
    let microsecond:f64 = 1_000.0;
    let millisecond     = microsecond * 1000.0;
    let second          = millisecond * 1000.0;
    let minute          = 60.0 * second;
    let hour            = 60.0 * minute;
    let day             = hour * 24.0;

    // Convert the time to nanoseconds

    let time = time * (1_000_000_000.0 / hz as f64);

    let unit;
    let scale;
    let has_plural;

    // Decide what units to use.

    if time >= day {
        unit       = "day";
        scale      = day;
        has_plural = true;
    } else if time >= hour {
        unit       = "hour";
        scale      = hour;
        has_plural = true;
    } else if time >= minute {
        unit       = "minute";
        scale      = minute;
        has_plural = true;
    } else if time >= second {
        unit       = "second";
        scale      = second;
        has_plural = true;
    } else if time >= millisecond {
        unit       = "ms";
        scale      = millisecond;
        has_plural = false;
    } else if time >= microsecond {
        unit       = "us";
        scale      = microsecond;
        has_plural = false;
    } else {
        unit       = "ns";
        scale      = 1.0;
        has_plural = false;
    }

    let plural = time != scale;

    let suffix =
        if plural & has_plural {
            "s"
        } else {
            ""
        };

    let     scaled_time = time / scale;
    let mut unit        = unit.to_string();

    unit.push_str(suffix);

    if scaled_time > 999999.0 {
        print_float_unit(name, scaled_time, &unit, printer);
    } else {
        let output = format!("    {:<12} {:>12.3} {}", name, scaled_time, unit);
        printer.print(&output);
    }
}

// Compute the sample variance.

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


// This function returns an array index to record a log value
// in a histogram.  Callers are expected to use two arrays,
// one for positive and one for negative values, so this routine
// ignores the sign of its input.
//
// The algorithm implements a simple log-like function of the
// absolute value of its input.  It is intended only for making
// histograms.
//
// The pseudo-log of most negative integers n is defined as -log(-n)
// to give a reasonable histogram structure.  The pseudo-log of
// i64::MIN is defined as 63 for convenience.  This routine always
// returns a positive index for an array, so the return value is
// pseudo-log(-n).  The calling routine handles the negation by using
// separate arrays for positive and negative pseudo-log values.
//
// In addition, pseudo-log(0) is defined as 0.
//

fn pseudo_log_index(value: i64) -> usize {
    let mut place = 1;
    let mut log   = 0;

    let absolute;

    if value == i64::MIN {
        return 63;
    } else if value < 0 {
        absolute = (-value) as u64;
    } else {
        absolute = value as u64;
    }

    while place < absolute && log < 63 {
        place *= 2;
        log   += 1;
    }

    log
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

// These structures and routines are common code for printing
// statistics.

struct Printable {
    n:          u64,
    min:        i64,
    max:        i64,
    log_mode:   i64,
    mean:       f64,
    variance:   f64,
    skewness:   f64,
    kurtosis:   f64,
}

fn print_integer(name: &str, value: i64, printer: &mut dyn Printer) {
    let output = format!("    {:<12} {:>12}", name, value);
    printer.print(&output);
}

fn print_float(name: &str, value: f64, printer: &mut dyn Printer) {
    print_float_unit(name, value, "", printer)
}

fn print_float_unit(name: &str, value: f64, unit: &str, printer: &mut dyn Printer) {
    assert!(!value.is_nan());

    let value = format!("{:+e}", value)
        .replace('e', " e+")
        .replace("e+-", " e-") ;

    let mantissa_digits = 8;
    let mut mantissa = Vec::with_capacity(mantissa_digits);

    for char in value.chars() {
        if char == ' ' {
            break;
        }

        mantissa.push(char);

        if mantissa.len() == 8 {
            break;
        }
    }

    while mantissa.len() < mantissa_digits {
        mantissa.push('0');
    }

    let mantissa: String = mantissa.into_iter().collect();
    let exponent         = value.split(' ').last().unwrap();
    let output           = format!("    {:<13}    {} {} {}", name, mantissa, exponent, unit);

    printer.print(&output);
}

// Print the common integer statistics as passed in a Printable structure.

fn print_common_integer(data: &Printable, printer: &mut dyn Printer) {
    print_integer("Count", data.n as i64, printer);

    if data.n > 0 {
        print_integer("Minumum",  data.min,      printer);
        print_integer("Maximum",  data.max,      printer);
        print_integer("Log Mode", data.log_mode, printer);
    }
}

// Print the common computed statistics as passed in a Printable structure.
// This includes values like the mean, which should be limited to an integer
// value.

fn print_common_float(data: &Printable, printer: &mut dyn Printer) {
    if data.n > 0 {
        print_float("Mean",     data.mean,            printer);
        print_float("Std Dev",  data.variance.sqrt(), printer);
        print_float("Variance", data.variance,        printer);
        print_float("Skewness", data.skewness,        printer);
        print_float("Kurtosis", data.kurtosis,        printer);
    }
}


// Implement a structure for the pseudo-log histograms.

#[derive(Clone)]
pub struct LogHistogram {
    pub negative:   [u64; 64],
    pub positive:   [u64; 64],
}

impl LogHistogram {
    pub fn new() -> LogHistogram {
        let negative: [u64; 64] = [0; 64];
        let positive: [u64; 64] = [0; 64];

        LogHistogram { negative, positive }
    }

    // Record a sample value.

    pub fn record(&mut self, sample: i64) {
        if sample < 0 {
            self.negative[pseudo_log_index(sample)] += 1;
        } else {
            self.positive[pseudo_log_index(sample)] += 1;
        }
    }

    // This helper function prints the negative buckets.

    fn print_negative(&self, printer: &mut dyn Printer) {
        // Skip printing buckets that would appear before the first non-zero bucket.
        // So find the non-zero bucket with the highest index in the array.

        let mut i = self.negative.len() - 1;

        while i > 0 && self.negative[i] == 0 {
            i -= 1;
        }

        // If there's nothing to print, just return.

        if i == 0 && self.negative[0] == 0 {
            return;
        }

        // Start printing from the highest-index non-zero row.

        let     start_index = ((i + 4) / 4) * 4 - 1;
        let mut i           = start_index + 4;
        let mut rows        = (start_index + 1) / 4;

        while rows > 0 {
            assert!(i >= 3 && i < self.negative.len());
            i -= 4;

            printer.print(&format!("  {:>3}:    {:>14}    {:>14}    {:>14}    {:>14}",
                -(i as i64) + 3,
                commas_u64(self.negative[i - 3]),
                commas_u64(self.negative[i - 2]),
                commas_u64(self.negative[i - 1]),
                commas_u64(self.negative[i])
            ));

            rows -= 1;
        }
    }

    // This helper function prints the positive buckets.

    fn print_positive(&self, printer: &mut dyn Printer) {
        let mut last = self.positive.len() - 1;

        while last > 0 && self.positive[last] == 0 {
            last -= 1;
        }

        let stop_index = last;
        let mut i = 0;

        while i <= stop_index {
            assert!(i <= self.positive.len() - 4);

            printer.print(&format!("  {:>3}:    {:>14}    {:>14}    {:>14}    {:>14}",
                i,
                commas_u64(self.positive[i]),
                commas_u64(self.positive[i + 1]),
                commas_u64(self.positive[i + 2]),
                commas_u64(self.positive[i + 3])));

            i += 4;
        }
    }

    // Find the most common "log" bucket

    pub fn log_mode(&self) -> isize {
        let mut mode: isize = 0;
        let mut max: u64 = 0;

        for i in 0..self.negative.len() {
            if self.negative[i] > max {
                mode = -(i as isize);
                max  = self.negative[i];
            }
        }

        for i in 0..self.positive.len() {
            if self.positive[i] > max {
                mode = i as isize;
                max  = self.positive[i];
            }
        }

        mode
    }

    pub fn print(&self, printer: &mut dyn Printer) {
        printer.print("  Log Histogram");
        self.print_negative(printer);
        printer.print("  -----------------------");
        self.print_positive(printer);
    }

    pub fn clear(&mut self) {
        self.negative = [0; 64];
        self.positive = [0; 64];
    }
}

impl Default for LogHistogram {
    fn default() -> Self {
        Self::new()
    }
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
    //   print          prints the statistics and pseudo-log histogram
    //                      The first argument is an optional title prefix.

    fn print(&self, printer: Option<PrinterBox>);   // print the statistics
    fn set_title(&mut self, title: &str);           // set the title for printing

    // For internal use only.
    fn set_id(&mut self, index: usize);
    fn id(&self)                          -> usize;
    fn equals(&self, other: &dyn Rustics) -> bool;
    fn generic(&self)                     -> &dyn Any;
    fn histo_log_mode(&self)              -> i64;
}

pub trait Histograms {
    fn log_histogram(&self) -> LogHistogram;
    fn print_histogram(&self);
}

// Define the implementation of a very simple running integer sample space.

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

impl RunningInteger {
    pub fn new(name_in: &str) -> RunningInteger {
        let id      = usize::MAX;
        let name    = String::from(name_in);
        let title   = String::from(name_in);
        let printer = stdout_printer();

        RunningInteger { name, title, id, count: 0, mean: 0.0, moment_2: 0.0, moment_3: 0.0,
            moment_4: 0.0, log_histogram: LogHistogram::new(), min: i64::MAX, max: i64::MIN,
            printer }
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

            self.mean     = new_mean;
            self.moment_2 = new_moment_2;
            self.moment_3 = new_moment_3;
            self.moment_4 = new_moment_4;

            self.min = std::cmp::min(self.min, sample);
            self.max = std::cmp::max(self.max, sample);
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

    fn print(&self, printer: Option<PrinterBox>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let printer = &mut *printer_box.lock().unwrap();

        let n        = self.count;
        let min      = self.min;
        let max      = self.max;
        let log_mode = self.log_histogram.log_mode() as i64;
        let mean     = self.mean;
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(&self.title);
        print_common_integer(&printable, printer);
        print_common_float(&printable, printer);
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

impl Histograms for RunningInteger {
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

    fn print(&self, printer: Option<PrinterBox>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
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

        printer.print(&self.title);
        print_common_integer(&printable, printer);
        print_common_float(&printable, printer);
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

impl Histograms for IntegerWindow {
    fn log_histogram(&self) -> LogHistogram {
        self.log_histogram.clone()
    }

    fn print_histogram(&self) {
        let printer = &mut *self.printer.lock().unwrap();
        self.log_histogram.print(printer);
    }
}

pub struct RunningTime {
    // name:               String,
    // title:              String,
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
    //   print          actually prints

    fn print(&self, printer: Option<PrinterBox>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let printer  = &mut *printer_box.lock().unwrap();
        let title    = &self.running_integer.title();
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
        print_common_integer_times(&printable, self.hz, printer);
        print_common_float_times(&printable, self.hz, printer);

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
    //   print          actually prints

    fn print(&self, printer: Option<PrinterBox>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let printer   = &mut *printer_box.lock().unwrap();

        let title     = self.integer_window.title();
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

        printer.print(&title);
        print_common_integer_times(&printable, self.hz, printer);
        print_common_float_times(&printable, self.hz, printer);

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
    //   print          prints the statistics and pseudo-log histogram
    //                      The first argument is an optional title prefix.
    //   set_title      sets the hierarchical title to be printed

    fn print(&self, printer: Option<PrinterBox>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let printer = &mut *printer_box.lock().unwrap();

        printer.print(&self.title);
        print_integer("Count", self.count, printer);
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
        let mut stats = RunningInteger::new(&"Test Statistics");

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        let printer = Arc::new(Mutex::new(TestPrinter { test_output: "test header ======" } ));
        stats.print(Some(printer));
    }

    pub fn test_simple_integer_window() {
        let window_size = 100;
        let mut stats = IntegerWindow::new(&"Test Statistics", window_size);

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        assert!(stats.log_mode() as usize == pseudo_log_index(stats.max_i64()));
        stats.print(None);
        let sample = 100;

        for _i in 0..2 * window_size {
            stats.record_i64(sample);
        }

        stats.print(None);
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

        time_stat.print(None);

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

        time_stat.print(None);

        // Get a sample with easily calculated summary statistics

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat = RunningTime::new("Test Running Time => 1..100", timer.clone());

        for i in 1..101 {
            setup_elapsed_time(&mut timer, i);
            time_stat.record_event();
        }

        time_stat.print(None);

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
            print_time(&header, time as f64, hz as i64, printer);

            time *= 10;
        }

        time_stat.print(None);
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

        time_stat.print(None);

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

        time_stat.print(None);

        // Get a sample with easily calculated summary statistics

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Time Window => 1..100", timer.clone());

        for i in 1..101 {
            setup_elapsed_time(&mut timer, i);
            time_stat.record_event();
        }

        time_stat.print(None);

        // Cover all the scales.

        let mut timer: TimerBox = Rc::from(RefCell::new(TestTimer::new(1_000_000_000)));
        let mut time_stat       = RunningTime::new("Test Time => Scale Test", timer.clone());

        let mut time    = 1;
        let     printer = &mut StdioPrinter::new(StreamKind::Stdout);

        for _i in 1..16 {
            setup_elapsed_time(&mut timer, time);
            time_stat.record_event();
            let header = format!("{} => ", commas_i64(time));
            print_time(&header, time as f64, hz as i64, printer);

            time *= 10;
        }

        time_stat.print(None);
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

        counter.print(None);
    }

    #[test]
    pub fn run_basic_tests() {
        test_commas();
        test_log_histogram();
        test_pseudo_log();
        test_simple_running_integer();
        test_simple_integer_window();
        test_simple_running_time();
        test_simple_time_window();
        test_simple_count();
    }
}
