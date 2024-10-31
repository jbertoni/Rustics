//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use super::Histogram;
use super::Printable;
use super::FloatHistogramBox;
use super::LogHistogramBox;
use super::Printer;
use super::biased_exponent;
use super::max_biased_exponent;
use super::exponent_bias;
use super::sign;

pub type HistoOption  = Option<HistoOpts>;

/// The HistoOpts struct is used to specify options on how to print
/// a histogram.

pub struct HistoOpts {
    pub merge_min:     isize,   // not yet implemented
    pub merge_max:     isize,   // not yet implemented
    pub no_zero_rows:  bool,    // suppress any rows that are all zeros
}

///
/// Float_histgram records a log-like histogram of f64 samples.
/// The numbers are broken into buckets based on the exponent,
/// broken in of 16.  For example, exponents 2^1 through 2^16
/// form one bucket.
///

pub struct FloatHistogram {
    negative:   Vec<u64>,
    positive:   Vec<u64>,
    buckets:    usize,
    nans:       usize,
    infinities: usize,
    samples:    usize,
    histo_opts: HistoOpts,
}

// Define how many exponent values are merged into one bucket.

fn bucket_divisor() -> isize {
    16
}

// Define the number of elements printed per row.  This actually
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

    pub fn new(histo_opts: HistoOpts) -> FloatHistogram {
        let buckets    = buckets() as usize;
        let buckets    = roundup(buckets, print_roundup());
        let negative   = vec![0; buckets];
        let positive   = vec![0; buckets];
        let samples    = 0;
        let nans       = 0;
        let infinities = 0;

        FloatHistogram { negative, positive, buckets, samples, nans, infinities, histo_opts }
    }

    ///  Records one f64 sample into its bucket.

    pub fn record(&mut self, sample: f64) {
        if sample.is_nan() {
            self.nans += 1;
            return;
        }

        // Get the index into the histogram.
        //
        // We have two separate arrays for positive and negative
        // values.

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

    /// Returns the start biased exponent of the bucket into
    /// which the value goes.  The sign of the value returned 
    /// matches the sign of the samples in the bucket.

    pub fn log_mode(&self) -> isize {
        let mut mode = 0;
        let mut max  = self.negative[0];

        for i in 1..self.negative.len() {
            if self.negative[i] > max {
                max  = self.negative[i];
                mode = -(i as isize);
            }
        }

        for i in 0..self.positive.len() {
            if self.positive[i] > max {
                max  = self.positive[i];
                mode = i as isize;
            }
        }

        mode * bucket_divisor()
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
                    format!("    -2^{:>5}:    {:>14}    {:>14}    {:>14}    {:>14}",
                        exponent,
                        Printable::commas_u64(self.negative[index    ]),
                        Printable::commas_u64(self.negative[index + 1]),
                        Printable::commas_u64(self.negative[index + 2]),
                        Printable::commas_u64(self.negative[index + 3])
                    );

                printer.print(&output);

                rows  -= 1;

                if index >= print_roundup() {
                    index -= 4;
                }
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
                    format!("    2^{:>5}:    {:>14}    {:>14}    {:>14}    {:>14}",
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

    /// Prints the histogrm.

    pub fn print(&self, printer: &mut dyn Printer) {
        self.histo_opts(printer, &self.histo_opts);
    }

    /// Prints the histogram.  The histo_opts option is not fully
    /// implemented.

    pub fn histo_opts(&self, printer: &mut dyn Printer, histo_opts: &HistoOpts) {
        let header =
            format!("  Log Histogram:  ({} NaN, {} infinite, {} samples)",
                self.nans, self.infinities, self.samples);

        printer.print(&header);
        self.print_negative(printer, histo_opts);
        printer.print("  -----------------------");
        self.print_positive(printer, histo_opts);
    }

    /// Deletes all data from the histogram.

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
}

impl Histogram for FloatHistogram {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.histo_opts(printer, &self.histo_opts);
    }

    /// Clears the histogram data.

    fn clear_histogram(&mut self) {
        self.clear()
    }

    /// Converts the self pointer to specific histogram types if
    /// possible.

    fn to_log_histogram  (&self) -> Option<LogHistogramBox> {
        None
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::stdout_printer;
    use crate::min_exponent;
    use super::*;

    fn simple_print_test() {
        let     merge_min    = min_exponent();
        let     merge_max    = min_exponent();
        let     no_zero_rows = true;
        let     histo_opts   = HistoOpts { merge_min, merge_max, no_zero_rows };
        let mut histogram    = FloatHistogram::new(histo_opts);
        let     max_index    = max_biased_exponent() / bucket_divisor();

        for i in 0..= max_index {
            histogram.negative[i as usize] = i as u64;
        }

        for i in 0..= max_index {
            histogram.positive[i as usize] = i as u64;
        }

        let     printer_box = stdout_printer();
        let     printer     = &mut *printer_box.lock().unwrap();

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

        println!("simple_print_test:  start sampling");

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

    #[test]
    fn run_tests() {
        simple_print_test()
    }
}
