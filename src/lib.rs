//
//  This code is available under the Berkeley 2-Clause license.  It is also available
//  as public domain source where permitted by law.
//

//! 'rustics' provides a very simple interface for
//!
//! ## Types
//!
//! * Recording Integers
//!
//!     * Integer statistics provide basic parameters, like the mean, and pseudo-log histogram.
//!     *  For the pseudo-log histogram, the pseudo-log of a negative number n is defines as -log(-n).
//!
//! * Integer staticstics
//!     * RunningInteger
//!
//!         * This type implements a few simple running statistics for i64 sample values.
//!
//!     * RunningWindow
//!
//!         * This type implements a fixed-size window of the last n samples recorded.
//!         * The histogram counts all samples seen, not just the current window.
//!
//! * Creating Sets
//!
//!     * The "sets" module implments a simple feature allowing the creation of sets that accept
//!       statistics and subsets as members.
//!
//!     * RusticsSet
//!         * This type functions as a container for sets and subsets that can be printed and
//!           cleared on command.
//!

use std::sync::Mutex;
use std::sync::Arc;
use std::any::Any;

pub mod arc_sets;
pub mod rc_sets;

pub type PrinterBox = Arc<Mutex<dyn Printer>>;

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
     *  handle leading signs that'll cause a comma to be added.  The condition
     * is a string length is 1 mod 3 and the top character is a sign.
     */
    if value.len() % comma_interval == 1 {
        match value.chars().next().unwrap() {
            '+' => { sign = "+"; digits = value[1..].to_string(); }
            '-' => { sign = "-"; digits = value[1..].to_string(); }
            _ => { sign = ""; digits = value.to_string(); }
        }
    } else {
        sign = "";
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
            _ => result,
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

pub fn compute_skewness(count: u64, moment_3: f64, moment_2: f64) -> f64 {
    if count < 2 || moment_2 == 0.0 {
        return 0.0;
    }

    let n = count as f64;
    let m3 = moment_3 / n;
    let m2 = moment_2 / n;
    let skewness = m3 / m2.powf(1.5);
    let correction = (n * (n - 1.0)).sqrt() / (n - 2.0);
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

    let n = count as f64;
    let kurtosis = moment_4 / (moment_2.powf(2.0) / n) - 3.0;
    let correction = (n - 1.0) / ((n - 2.0) * (n - 3.0));
    let kurtosis_factor = (n + 1.0) * kurtosis + 6.0;
    let sample_excess_kurtosis = correction * kurtosis_factor;

    sample_excess_kurtosis
}

// This implements a simple log-like function of the absolute
// value of its input.  It is intended only for making histograms.
//
// The log of a negative integer n is defined as -log(-n) to give
// a reasonable histogram structure.

fn integer_log(value: i64) -> usize {
    let mut place = 1;
    let mut log = 0;

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
        log += 1;
    }

    log
}

// Insert a delimiter and concatenate the parent and child names  when creating
// a hierarchical title.

fn create_title(title_prefix: &str, title: &str) -> String {
    let title =
        if title_prefix.is_empty() {
            // let mut full_title = String::from(title_prefix).to_owned();
            let mut full_title = String::from(title_prefix);
            full_title.push_str(" ==> ");
            full_title.push_str(title);
            full_title
        } else {
            title.to_string()
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
    let exponent = value.split(' ').last().unwrap();
    let output = format!("    {:<13}    {} {}", name, mantissa, exponent);

    printer.print(&output);
}

// Print the common integer statistics as passed in a Printable structure.

fn print_common_integer(data: &Printable, printer: &mut dyn Printer) {
    print_integer("Count", data.n as i64, printer);
    print_integer("Minumum", data.min, printer);
    print_integer("Maximum", data.max, printer);
    print_integer("Log Mode", data.log_mode, printer);
}

// Print the common computed statistics as passed in a Printable structure.
// This includes values like the mean, which should be limited to aninteger
// value.

fn print_common_float(data: &Printable, printer: &mut dyn Printer) {
    print_float("Mean", data.mean, printer);
    print_float("Std Dev", data.variance.sqrt(), printer);
    print_float("Variance", data.variance, printer);
    print_float("Skewness", data.skewness, printer);
    print_float("Kurtosis", data.kurtosis, printer);
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
            self.negative[integer_log(sample)] += 1;
        } else {
            self.positive[integer_log(sample)] += 1;
        }
    }

    // This helper function prints the negative buckets.

    fn print_negative(&self, printer: &mut dyn Printer) {
        let mut i = self.negative.len() - 1;

        while i > 0 && self.negative[i] == 0 {
            i -= 1;
        }

        if i == 0 && self.negative[0] == 0 {
            return;
        }

        let start_index = ((i + 4) / 4) * 4 - 1;
        let mut i = start_index + 4;
        let mut rows = (start_index + 1) / 4;

        while rows > 0 {
            i -= 4;

            if i >= 64 {
                printer.print(&format!("  {:>3}:    {:>14}", -64, self.negative[64]));
            } else {
                printer.print(&format!("  {:>3}:    {:>14}    {:>14}    {:>14}    {:>14}",
                    -(i as i64) + 3,
                    commas_u64(self.negative[i - 3]),
                    commas_u64(self.negative[i - 2]),
                    commas_u64(self.negative[i - 1]),
                    commas_u64(self.negative[i])
                    ));
            }

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
            if i < 64 {
                printer.print(&format!("  {:>3}:    {:>14}    {:>14}    {:>14}    {:>14}",
                    i,
                    commas_u64(self.positive[i]),
                    commas_u64(self.positive[i + 1]),
                    commas_u64(self.positive[i + 2]),
                    commas_u64(self.positive[i + 3])));
            } else {
                printer.print(&format!("  {:>3}:    {:>14}", 64, self.positive[64]));
            }

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
                max = self.negative[i];
            }
        }

        for i in 0..self.positive.len() {
            if self.positive[i] > max {
                mode = i as isize;
                max = self.positive[i];
            }
        }

        mode
    }

    pub fn print(&self, printer: &mut dyn Printer) {
        printer.print("Log Histogram");
        self.print_negative(printer);
        printer.print(" -----------------------");
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

pub trait Printer {
    fn print(&self, output: &str);
}

// Define a printer that will send output to stdout or stderr, as
// configured.

pub struct StdioPrinter {
    which: String,
}

impl StdioPrinter {
    pub fn new(which: &str) -> StdioPrinter {
        let which = which.to_string();
        StdioPrinter { which }
    }
}

impl Printer for StdioPrinter {
    fn print(&self, output: &str) {
        match self.which.as_str() {
            "stdout" => println!("{}", output),
            "stderr" => eprintln!("{}", output),
            _ => println!("{}", output),
        }
    }
}

// Define the main trait for collecting statistics.  Eventually, this
// library will support floating point samples.

pub trait Rustics {
    fn record_i64(&mut self, sample: i64);  // add an i64 sample
    fn record_f64(&mut self, sample: f64);  // add an f64 sample -- not implemented

    fn name(&self) -> String;               // a text (UTF-8) name to print
    fn class(&self) -> &str;                // the type of a sample:  integer or floating
    fn count(&self) -> u64;                 // the current sample count
    fn log_mode(&self) -> isize;            // the most common pseudo-log
    fn mean(&self) -> f64;
    fn standard_deviation(&self) -> f64;
    fn variance(&self) -> f64;
    fn skewness(&self) -> f64;
    fn kurtosis(&self) -> f64;

    fn int_extremes(&self) -> bool;         // does this statistic implement integer extremes?
    fn min_i64(&self) -> i64;
    fn min_f64(&self) -> f64;
    fn max_i64(&self) -> i64;
    fn max_f64(&self) -> f64;

    fn precompute(&mut self);               // precompute the various statistics for printing
    fn clear(&mut self);                    // clear all the statistics

    // Functions for printing
    //   print          actually prints
    //   set_printer    supplies a callback to print a single line

    fn print(&self, title_prefix: &str);
    fn set_printer(&mut self, printer: PrinterBox);

    // For internal use only.
    fn set_id(&mut self, index: usize);
    fn id(&self) -> usize;
    fn equals(&self, other: &dyn Rustics) -> bool;
    fn generic(&self) -> &dyn Any;
}

pub trait Histograms {
    fn log_histogram(&self) -> LogHistogram;
    fn print_histogram(&self);
}

// Define the implementation of a very simple running integer sample space.

pub struct RunningInteger {
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
    pub fn new(title: &str) -> RunningInteger {
        let id = usize::max_value();
        let title = String::from(title);
        let which = "stdout".to_string();
        let printer = Arc::new(Mutex::new(StdioPrinter { which }));

        RunningInteger { title, id, count: 0, mean: 0.0, moment_2: 0.0, moment_3: 0.0,
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
            self.mean = sample_f64;
            self.moment_2 = 0.0;
            self.moment_3 = 0.0;
            self.moment_4 = 0.0;
            self.min = sample;
            self.max = sample;
        } else {
            let distance_mean = sample_f64 - self.mean;
            let new_mean = self.mean + distance_mean / self.count as f64;
            let distance_new_mean = sample_f64 - new_mean;
            let square_estimate = distance_mean * distance_new_mean;
            let cube_estimate = square_estimate * square_estimate.sqrt();
            let new_moment_2 = self.moment_2 + square_estimate;
            let new_moment_3 = self.moment_3 + cube_estimate;
            let new_moment_4 = self.moment_4 + square_estimate * square_estimate;

            self.mean = new_mean;
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

    fn name(&self) -> String {
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
        self.count = 0;
        self.mean = 0.0;
        self.moment_2 = 0.0;
        self.moment_3 = 0.0;
        self.moment_4 = 0.0;
        self.min = i64::MIN;
        self.max = i64::MAX;

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

    fn print(&self, title_prefix: &str) {
        let printer = &mut *self.printer.lock().unwrap();

        let title = &create_title(title_prefix, &self.title);
        let n = self.count;
        let min = self.min;
        let max = self.max;
        let log_mode = self.log_histogram.log_mode() as i64;
        let mean = self.mean;
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(title);
        print_common_integer(&printable, printer);
        print_common_float(&printable, printer);
        self.log_histogram.print(printer);
    }

    fn set_printer(&mut self, printer: PrinterBox) {
        self.printer = printer;
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
    title:          String,
    index:          usize,
    window_size:    usize,
    vector:         Vec<i64>,
    id:             usize,
    sample_count:   u64,

    stats_valid:    bool,
    mean:           f64,
    sum:            f64,
    moment_2:       f64,
    moment_3:       f64,
    moment_4:       f64,

    pub log_histogram:  LogHistogram,

    printer:        PrinterBox,
}

struct Crunched {
    sum:        f64,
    moment_2:   f64,
    moment_3:   f64,
    moment_4:   f64,
}

impl Crunched {
    pub fn _new() -> Crunched {
        let sum = 0.0;
        let moment_2 = 0.0;
        let moment_3 = 0.0;
        let moment_4 = 0.0;

        Crunched { sum, moment_2, moment_3, moment_4 }
    }
}

impl IntegerWindow {
    pub fn new(title: &str, window_size: usize) -> IntegerWindow {
        if window_size == 0 {
            panic!("The window size is zero.");
        }

        let title = String::from(title);
        let id = usize::max_value();
        let vector = Vec::with_capacity(window_size);
        let index = 0;
        let sample_count = 0;
        let stats_valid = false;
        let mean = 0.0;
        let sum = 0.0;
        let moment_2 = 0.0;
        let moment_3 = 0.0;
        let moment_4 = 0.0;
        let log_histogram = LogHistogram::new();
        let which = "stdout".to_string();
        let printer = Arc::new(Mutex::new(StdioPrinter { which }));

        IntegerWindow { title, id, window_size, vector, index, sample_count, stats_valid,
            mean, sum, moment_2, moment_3, moment_4, log_histogram, printer }
    }

    fn sum(&self) -> f64 {
        let mut sum = 0.0;

        for sample in self.vector.iter() {
            sum += *sample as f64;
        }

        sum
    }

    fn crunch(&self) -> Crunched {
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
            let square = distance * distance;
            moment_2 += distance * distance;
            moment_3 += distance * square;
            moment_4 += square * square;
        }

        Crunched { sum, moment_2, moment_3, moment_4 }
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
            self.sample_count += 1;
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

    fn name(&self) -> String {
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
        if self.sample_count == 0 {
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
        if self.sample_count == 0 || self.stats_valid {
            return;
        }

        let crunched = self.crunch();

        self.mean = crunched.sum / self.vector.len() as f64;
        self.sum = crunched.sum;
        self.moment_2 = crunched.moment_2;
        self.moment_3 = crunched.moment_3;
        self.moment_4 = crunched.moment_4;
        self.stats_valid = true;
    }

    fn clear(&mut self) {
        self.vector.clear();
        self.index = 0;
        self.stats_valid = false;
    }

    fn print(&self, title_prefix: &str) {
        let printer = &mut *self.printer.lock().unwrap();
        let title = &create_title(title_prefix, &self.title);
        let n = self.vector.len() as u64;

        let min = self.compute_min();
        let max = self.compute_max();
        let log_mode = self.log_histogram.log_mode() as i64;

        let mean;
        let variance;
        let skewness;
        let kurtosis;

        if self.stats_valid {
            mean = self.mean();
            variance = self.variance();
            skewness = self.skewness();
            kurtosis = self.kurtosis();
        } else {
            let crunched = self.crunch();

            mean = crunched.sum / n as f64;
            variance = compute_variance(n, crunched.moment_2);
            skewness = compute_skewness(n, crunched.moment_2, crunched.moment_3);
            kurtosis = compute_kurtosis(n, crunched.moment_2, crunched.moment_4);
        }

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(&format!(" *** {}", title));
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

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn set_printer(&mut self, printer: PrinterBox) {
        self.printer = printer;
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let printer = &mut TestPrinter { test_output: &"Test Output" };

        let test = [ 1, -1, 4, 25, 4109, -4108, -8, -9, -16, -17, 3, 8, 16 ];

        for i in test.iter() {
            histogram.record(*i);
        }

        histogram.print(printer);
    }

    pub fn test_integer_log() {
        let test = [ 1,   0, -1, -4, -3, i64::MIN, 3, 4, 5, 8, i64::MAX ];
        let expect = [ 0, 0,  0,  2,  2,       63, 2, 2, 3, 3,       63 ];

        let mut i = 0;

        for sample in test.iter() {
            println!("integer_log({}) = {}", *sample, integer_log(*sample));
            assert_eq!(integer_log(*sample), expect[i]);
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

        stats.print(&"");

        stats.set_printer(Arc::new(Mutex::new(TestPrinter { test_output: &"RunningInteger Output" })));
        stats.print(&"Dynamic Printing");
    }

    pub fn test_simple_integer_window() {
        let window_size = 100;
        let mut stats = IntegerWindow::new(&"Test Statistics", window_size);
        stats.set_printer(Arc::new(Mutex::new(TestPrinter { test_output: &"IntegerWindow Output" })));

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        assert!(stats.log_mode() as usize == integer_log(stats.max_i64()));
        stats.print(&"");
        let sample = 100;

        for _i in 0..2 * window_size {
            stats.record_i64(sample);
        }

        stats.print(&"");
        assert!(stats.mean() == sample as f64);
        assert!(stats.log_mode() as usize == integer_log(sample));
    }

    #[test]
    pub fn run_basic_tests() {
        test_commas();
        test_log_histogram();
        test_integer_log();
        test_simple_running_integer();
        test_simple_integer_window();
    }
}
