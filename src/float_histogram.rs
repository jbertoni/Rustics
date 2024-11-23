//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//!
//! ## Type
//! * FloatHistogram
//!   * FloatHistogram provides a very coarse log histogram that is similar to
//!     the LogHistogram type with its pseudo-log function.
//!
//!   * Samples are divided into buckets based on their sign and exponent.
//!     There is one bucket per 16 exponents, and separate buckets for positive
//!     and negative samples with the same exponent.
//!
//!   * NaNs are counted separately, and otherwise are ignored.
//!
//!   * f64::INFINITY samples go into the largest bucket, and into a count of
//!     infinite values.
//!
//!   * f64::NEG_INFINITY samples go into the smallest bucket, and into a count
//!     of infinite values.
//!
//! ## Example
//!     use rustics::float_histogram::FloatHistogram;
//!     use rustics::float_histogram::bucket_divisor;
//!     use rustics::exponent_bias;
//!     use rustics::PrintOpts;
//!     use rustics::float_histogram::HistoOpts;
//!     use rustics::stdout_printer;
//!     use rustics::printer_mut;
//!
//!     // Create a HistOp for new().
//!
//!     let merge_min    = 0;  // not implemented yet
//!     let merge_max    = 0;  // not implemented yet
//!     let no_zero_rows = false;
//!
//!     let histo_opts = HistoOpts { merge_min, merge_max, no_zero_rows };
//!     let histo_opts = Some(histo_opts);
//!     let printer    = None;
//!     let title      = None;
//!     let units      = None;
//!     let print_opts = PrintOpts { printer, title, units, histo_opts };
//!
//!     // Create a histogram and accept the default output format.
//!
//!     let mut histogram = FloatHistogram::new(&Some(print_opts));
//!
//!     let sample_count = 1000;
//!
//!     for i in 0..sample_count {
//!          histogram.record(-(i as f64));
//!     }
//!
//!     // Create a Printer instance for output.
//!
//!     let printer = stdout_printer();
//!     let printer = printer_mut!(printer);
//!
//!     histogram.print(printer);
//!
//!     assert!(histogram.samples     == sample_count as usize);
//!     assert!(histogram.nans        == 0);
//!     assert!(histogram.infinities  == 0);
//!
//!     // Values -0.0 and -1.0 should be in the same bucket.
//!
//!     let zero_bucket = exponent_bias() / bucket_divisor();
//!     let zero_bucket = zero_bucket as usize;
//!
//!     assert!(histogram.negative[zero_bucket    ] == 2);
//!     assert!(histogram.negative[zero_bucket + 1] == sample_count - 2);
//!
//!     // Now test some non-finite values.  NaN values do not
//!     // go into the sample count.
//!
//!     histogram.record(f64::INFINITY);
//!     histogram.record(f64::NEG_INFINITY);
//!     histogram.record(f64::NAN);
//!
//!     assert!(histogram.nans       == 1);
//!     assert!(histogram.infinities == 2);
//!     assert!(histogram.samples    == sample_count as usize + 2);
//!
//!     histogram.print(printer);

use super::Histogram;
use super::Printable;
use super::FloatHistogramBox;
use super::PrintOption;
use super::LogHistogramBox;
use super::Printer;
use super::biased_exponent;
use super::max_biased_exponent;
use super::exponent_bias;
use super::sign;
use super::parse_histo_opts;

/// The HistoOpts struct is used to specify options on how to print
/// a histogram.

#[derive(Clone, Copy)]
pub struct HistoOpts {
    pub merge_min:     isize,   // not yet implemented
    pub merge_max:     isize,   // not yet implemented
    pub no_zero_rows:  bool,    // suppress any rows that are all zeros
}

impl Default for HistoOpts {
    fn default() -> HistoOpts {
        let merge_min    = 0;
        let merge_max    = 0;
        let no_zero_rows = false;

        HistoOpts { merge_min, merge_max, no_zero_rows }
    }
}

///
/// FloatHistogram records a log-like histogram of f64 samples.
/// The numbers are recorded into buckets based on the exponent,
/// broken into groups of 16.  For example, exponents 2^1 through
/// 2^16 form one bucket.
///

pub struct FloatHistogram {
    pub negative:   Vec<u64>,
    pub positive:   Vec<u64>,
    pub buckets:    usize,
    pub nans:       usize,
    pub infinities: usize,
    pub samples:    usize,
    pub print_opts: PrintOption,
    pub histo_opts: HistoOpts,
}

/// Defines how many exponent values are merged into one bucket.

pub fn bucket_divisor() -> isize {
    16
}

// Define the number of counts printed per row.  This actually
// is hard-coded in the actual format statement.

fn print_roundup() -> usize {
    4
}

// Compute the number of buckets for the negative and positive
// arrays.

fn buckets() -> isize {
    max_biased_exponent() / bucket_divisor()
}

// Do covered division.

fn roundup(value: usize, multiple: usize) -> usize {
    ((value + multiple - 1) / multiple) * multiple
}

impl FloatHistogram {
    /// Creates a new histogram.  The histo_opts option currently is
    /// only partially implemented.

    pub fn new(print_opts: &PrintOption) -> FloatHistogram {
        let buckets    = buckets() as usize;
        let buckets    = roundup(buckets, print_roundup());
        let negative   = vec![0; buckets];
        let positive   = vec![0; buckets];
        let samples    = 0;
        let nans       = 0;
        let infinities = 0;
        let histo_opts = parse_histo_opts(print_opts);
        let print_opts = print_opts.clone();

        FloatHistogram {
            negative, positive, buckets, samples, nans, infinities, print_opts, histo_opts
        }
    }

    /// Records one f64 sample into its bucket.

    pub fn record(&mut self, sample: f64) {
        //  NaN values are counted but otherwise ignored.

        if sample.is_nan() {
            self.nans += 1;
            return;
        }

        // Get the index into the histogram.
        //
        // We have two separate arrays for positive and negative
        // values, so keep track of the sign.

        let index =
            if sample.is_infinite() {
                self.infinities += 1;

                let index = max_biased_exponent() / bucket_divisor();

                index as usize
            } else if sample == 0.0 {
                let index = exponent_bias() / bucket_divisor();

                index as usize
            } else {
                let index = biased_exponent(sample) / bucket_divisor();

                index as usize
            };

        let sign = sign(sample);

        // Now index into the appropriate array.

        if sign < 0 {
            self.negative[index] += 1;
        } else {
            self.positive[index] += 1;
        }

        self.samples += 1;
    }

    /// This function returns the biased IEEE binary64
    /// exponent, with the sign of the sample value
    /// used as a sign for the result.
    ///

    pub fn convert_log_mode(&self) -> (isize, isize) {
        let mut mode = 0;
        let mut sign = -1;
        let mut max  = self.negative[0];

        for i in 1..self.negative.len() {
            if self.negative[i] > max {
                max  = self.negative[i];
                mode = i as isize;
            }
        }

        for i in 0..self.positive.len() {
            if self.positive[i] > max {
                max  = self.positive[i];
                mode = i as isize;
                sign = 1;
            }
        }

        let biased_exponent = mode * bucket_divisor();
        let biased_exponent = biased_exponent + bucket_divisor() / 2;

        (sign, biased_exponent - exponent_bias())
    }

    pub fn mode_value(&self) -> f64 {
        let (sign, exponent) = self.convert_log_mode();

        let result   = 2.0_f64;
        let result   = result.powi(exponent as i32);
        let result   = result - result / 4.0;

        sign as f64 * result
    }

    // This helper method prints the negative buckets.

    fn print_negative(&self, printer: &mut dyn Printer, histo_opts: &HistoOpts) {
        // Skip printing buckets that would appear before the first non-zero bucket.
        // So find the non-zero bucket with the highest index in the array.

        let mut scan = self.negative.len() - 1;

        while scan > 0 && self.negative[scan] == 0 {
            scan -= 1;
        }

        // If there's nothing to print, just return.

        if scan == 0 && self.negative[0] == 0 {
            return;
        }

        // Start printing from the lowest-index row.

        let     start_row   = scan / print_roundup();
        let mut rows        = start_row + 1;
        let mut index       = start_row * print_roundup();

        while rows > 0 {
            if
                histo_opts.no_zero_rows
            ||  self.negative[index    ] != 0
            ||  self.negative[index + 1] != 0
            ||  self.negative[index + 2] != 0
            ||  self.negative[index + 3] != 0 {

                let exponent = (index as isize) * bucket_divisor();
                let exponent = exponent - exponent_bias();

                assert!(print_roundup() == 4);    // This format assumes a

                let output =
                    format!("    -2^{:>5}:    {:>10}    {:>10}    {:>10}    {:>10}",
                        exponent,
                        Printable::commas_u64(self.negative[index    ]),
                        Printable::commas_u64(self.negative[index + 1]),
                        Printable::commas_u64(self.negative[index + 2]),
                        Printable::commas_u64(self.negative[index + 3])
                    );

                printer.print(&output);
            }

            rows -= 1;

            if index >= print_roundup() {
                index -= 4;
            }
        }
    }

    // This helper method prints the positive buckets.

    fn print_positive(&self, printer: &mut dyn Printer, histo_opts: &HistoOpts) {
        if self.samples == 0 {
            return;
        }

        let mut last = self.buckets - 1;

        while last > 0 && self.positive[last] == 0 {
            last -= 1;
        }

        let     stop_index = last;
        let mut i          = 0;

        assert!(print_roundup() == 4);    // This code assumes len() % 4 == 0

        // Print the rows that have non-zero entries.  Each row has
        // the sample counts for 4 buckets.

        while i <= stop_index {
            assert!(i <= self.positive.len() - 4);

            if
                histo_opts.no_zero_rows
            ||  self.positive[i    ] != 0
            ||  self.positive[i + 1] != 0
            ||  self.positive[i + 2] != 0
            ||  self.positive[i + 3] != 0 {

                let exponent = i as isize * bucket_divisor();
                let exponent = exponent - exponent_bias();

                let output =
                    format!("    2^{:>5}:    {:>10}    {:>10}    {:>10}    {:>10}",
                        exponent,
                        Printable::commas_u64(self.positive[i]    ),
                        Printable::commas_u64(self.positive[i + 1]),
                        Printable::commas_u64(self.positive[i + 2]),
                        Printable::commas_u64(self.positive[i + 3])
                    );

                printer.print(&output);
            }

            i += 4;
        }
    }

    /// Prints the histogram.

    pub fn print(&self, printer: &mut dyn Printer) {
        self.print_opts(printer, &self.histo_opts);
    }

    /// Prints the histogram with the options specified.  The HistoOpts struct
    //  is not fully implemented.

    pub fn print_opts(&self, printer: &mut dyn Printer, histo_opts: &HistoOpts) {
        let header =
            format!("  Float Histogram:  ({} NaN, {} infinite, {} samples)",
                self.nans, self.infinities, self.samples);

        printer.print(&header);
        self.print_negative(printer, histo_opts);
        printer.print("  -----------------------");
        self.print_positive(printer, histo_opts);
    }

    /// Resets the histogram to its initial state.

    pub fn clear(&mut self) {
        self.negative   = vec![0; self.buckets];
        self.positive   = vec![0; self.buckets];
        self.samples    = 0;
        self.nans       = 0;
        self.infinities = 0;
    }

    /// Returns the number of samples that were NaN and the number that
    /// were non-finite.

    pub fn non_finites(&self) -> (usize, usize) {
        (self.nans, self.infinities)
    }

    pub fn histo_opts(&self) -> HistoOpts {
        self.histo_opts
    }

    pub fn equals(&self, other: &FloatHistogram) -> bool {
        for i in 0..other.negative.len() {
            if self.negative[i] != other.negative[i] {
                return false;
            }
        }

        for i in 0..other.positive.len() {
            if self.positive[i] != other.positive[i] {
                return false;
            }
        }

        if self.samples != other.samples {
            return false;
        }

        if self.nans != other.nans {
            return false;
        }

        if self.infinities != other.infinities {
            return false;
        }

        true
    }
}

impl Histogram for FloatHistogram {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.print_opts(printer, &self.histo_opts);
    }

    /// Clears the histogram data.

    fn clear_histogram(&mut self) {
        self.clear()
    }

    /// Returns None since this histogram is not in a box.

    fn to_log_histogram  (&self) -> Option<LogHistogramBox> {
        None
    }

    /// Returns None since this histogram is not in a box.

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::stdout_printer;
    use crate::min_exponent;
    use crate::PrintOpts;
    use crate::printer_mut;
    use super::*;

    fn simple_test() {
        let     merge_min    = min_exponent();
        let     merge_max    = min_exponent();
        let     no_zero_rows = true;
        let     printer      = None;
        let     title        = None;
        let     units        = None;
        let     histo_opts   = HistoOpts { merge_min, merge_max, no_zero_rows };
        let     histo_opts   = Some(histo_opts);
        let     print_opts   = PrintOpts { printer, title, units, histo_opts };
        let mut histogram    = FloatHistogram::new(&Some(print_opts));
        let     max_index    = max_biased_exponent() / bucket_divisor();

        for i in 0..= max_index {
            histogram.negative[i as usize] = i as u64;
        }

        for i in 0..= max_index {
            histogram.positive[i as usize] = i as u64;
        }

        let printer_box = stdout_printer();
        let printer     = printer_mut!(printer_box);

        histogram.print(printer);

        histogram.clear();

        for data in &histogram.negative {
            assert!(*data == 0);
        }

        for data in &histogram.positive {
            assert!(*data == 0);
        }

        assert!(histogram.samples    == 0);
        assert!(histogram.nans       == 0);
        assert!(histogram.infinities == 0);

        histogram.print(printer);

        let sample_count = 1000;

        for i in 0..sample_count {
            histogram.record(-(i as f64));
        }

        histogram.print(printer);

        assert!(histogram.samples     == sample_count as usize);
        assert!(histogram.nans        == 0);
        assert!(histogram.infinities  == 0);

        // Values -0.0 and -1.0 should be in the same bucket.

        let zero_bucket = exponent_bias() / bucket_divisor();
        let zero_bucket = zero_bucket as usize;

        assert!(histogram.negative[zero_bucket    ] == 2);
        assert!(histogram.negative[zero_bucket + 1] == sample_count - 2);

        for i in 0..sample_count {
            histogram.record(i as f64);
        }

        histogram.print(printer);

        assert!(histogram.samples     == 2 * sample_count as usize);
        assert!(histogram.nans        == 0);
        assert!(histogram.infinities  == 0);

        assert!(histogram.positive[zero_bucket    ] == 2);
        assert!(histogram.positive[zero_bucket + 1] == sample_count - 2);

        histogram.record(f64::INFINITY);
        histogram.record(f64::NEG_INFINITY);
        histogram.record(f64::NAN);

        histogram.print(printer);

        let index = max_biased_exponent() / bucket_divisor();
        let index = index as usize;

        assert!(histogram.positive[index] == 1);
        assert!(histogram.positive[index] == 1);

        assert!(histogram.samples == (2 * sample_count + 2) as usize);

        assert!(histogram.nans       == 1);
        assert!(histogram.infinities == 2);
    }

    fn test_documentation() {
        // Create a HistOp for new().

        let merge_min    = 10;  // not implemented yet
        let merge_max    = 11;  // not implemented yet
        let no_zero_rows = false;
        let histo_opts   = HistoOpts { merge_min, merge_max, no_zero_rows };
        let printer      = None;
        let title        = None;
        let units        = None;
        let histo_opts   = Some(histo_opts);
        let print_opts   = PrintOpts { printer, title, units, histo_opts };

        // Create a histogram and accept the default output format.

        let mut histogram = FloatHistogram::new(&Some(print_opts));

        let sample_count = 1000;

        for i in 0..sample_count {
             histogram.record(-(i as f64));
        }

        // Create a Printer instance for output.

        let printer_box = stdout_printer();
        let printer     = printer_mut!(printer_box);

        histogram.print(printer);

        assert!(histogram.samples     == sample_count as usize);
        assert!(histogram.nans        == 0);
        assert!(histogram.infinities  == 0);

        // Values -0.0 and -1.0 should be in the same bucket.

        let zero_bucket = exponent_bias() / bucket_divisor();
        let zero_bucket = zero_bucket as usize;

        assert!(histogram.negative[zero_bucket    ] == 2);
        assert!(histogram.negative[zero_bucket + 1] == sample_count - 2);

        // Now test some non-finite values.  NaN values do not
        // go into the sample count.

        histogram.record(f64::INFINITY);
        histogram.record(f64::NEG_INFINITY);
        histogram.record(f64::NAN);

        assert!(histogram.nans       == 1);
        assert!(histogram.infinities == 2);
        assert!(histogram.samples    == sample_count as usize + 2);

        // Check the official interface.

        let (nans, infinities) = histogram.non_finites();

        assert!(nans       == 1);
        assert!(infinities == 2);

        // Check the Histogram trait.

        histogram.print_histogram(printer);
        histogram.clear_histogram();

        let (nans, infinities) = histogram.non_finites();

        assert!(nans       == 0);
        assert!(infinities == 0);

        // Check histo_opts().

        let histo_opts = histogram.histo_opts();

        assert!(histo_opts.merge_min    == merge_min);
        assert!(histo_opts.merge_max    == merge_max);
        assert!(histo_opts.no_zero_rows == no_zero_rows);
    }

    fn test_log_mode() {
        // Create a HistOp for new().

        let merge_min    = 0;  // not implemented yet
        let merge_max    = 0;  // not implemented yet
        let no_zero_rows = false;
        let histo_opts   = HistoOpts { merge_min, merge_max, no_zero_rows };

        let printer      = None;
        let title        = None;
        let units        = None;
        let histo_opts   = Some(histo_opts);
        let print_opts   = PrintOpts { printer, title, units, histo_opts };

        // Create a histogram and accept the default output format.

        let mut histogram = FloatHistogram::new(&Some(print_opts));

        let sample_count = 1000;

        for i in 0..sample_count {
             histogram.record(-(i as f64));
        }

        let (sign, exponent) = histogram.convert_log_mode();

        let sign     = sign as f64;
        let value    = 2_f64.powi(exponent as i32);
        let value    = value - value / 4.0;
        let expected = sign * value;

        let log_mode = histogram.mode_value();

        println!("test_log_mode:  got {}, expected {}", log_mode, expected);
        assert!(log_mode == expected);
    }

    fn test_float_equals() {
        let mut histo_1 = FloatHistogram::new(&None);
        let mut histo_2 = FloatHistogram::new(&None);

        for i in 0..1000 {
            let sample = i as f64;

            histo_1.record( sample);
            histo_1.record(-sample);
            histo_2.record( sample);
            histo_2.record(-sample);
        }

        assert!(histo_1.equals(&histo_2));

        histo_1.positive[1] += 1;
        assert!(! histo_1.equals(&histo_2));
        histo_1.positive[1] -= 1;
        assert!(histo_1.equals(&histo_2));

        histo_1.negative[1] += 1;
        assert!(! histo_1.equals(&histo_2));
        histo_1.negative[1] -= 1;
        assert!(histo_1.equals(&histo_2));

        histo_1.samples += 1;
        assert!(! histo_1.equals(&histo_2));
        histo_1.samples -= 1;
        assert!(histo_1.equals(&histo_2));

        histo_1.nans += 1;
        assert!(! histo_1.equals(&histo_2));
        histo_1.nans -= 1;
        assert!(histo_1.equals(&histo_2));

        histo_1.infinities += 1;
        assert!(! histo_1.equals(&histo_2));
        histo_1.infinities -= 1;
        assert!(histo_1.equals(&histo_2));
    }

    #[test]
    #[should_panic]
    fn test_to_log() {
        let histogram = FloatHistogram::new(&None);

        let _ = histogram.to_log_histogram().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_to_float() {
        let histogram = FloatHistogram::new(&None);

        let _ = histogram.to_float_histogram().unwrap();
    }

    #[test]
    fn run_tests() {
        simple_test();
        test_documentation();
        test_log_mode();
        test_float_equals();
    }
}
