//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::cmp::min;
use super::Histogram;
use super::Printable;
use super::FloatHistogramBox;
use super::LogHistogramBox;
use super::Printer;
use super::biased_exponent;
use super::min_exponent;
use super::max_exponent;
use super::max_biased_exponent;
use super::exponent_bias;

pub type SuppressOption  = Option<Suppress>;

pub struct Suppress {
    pub min:  isize,
    pub max:  isize,
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
    count:      usize,
    nans:       usize,
    infinities: usize,
    suppress:   Suppress,
}

fn bucket_divisor() -> isize {
    16
}

fn print_roundup() -> usize {
    4
}

fn buckets() -> isize {
    max_biased_exponent() / bucket_divisor()
}

fn roundup(count: usize, multiple: usize) -> usize {
    ((count + multiple - 1) / multiple) * multiple
}

impl FloatHistogram {
    /// Creates a new histogram.  The suppress option currently is
    /// unlimited.

    pub fn new(suppress: Suppress) -> FloatHistogram {
        let count      = buckets() as usize;
        let count      = roundup(count, print_roundup());
        let negative   = vec![0; count];
        let positive   = vec![0; count];
        let nans       = 0;
        let infinities = 0;

        FloatHistogram { negative, positive, count, nans, infinities, suppress }
    }

    ///  Records one f64 sample into its bucket.

    pub fn record(&mut self, sample: f64) {
        if sample.is_nan() {
            self.nans += 1;
            return;
        }

        // Get the index into the histogram.  This code ignores the sign of
        // the number.  We have two separate arrays for positive and negative
        // values.

        let index =
            if sample.is_infinite() {
                self.infinities += 1;

                let index = max_biased_exponent() / bucket_divisor();

                index as usize
            } else {
                let index = biased_exponent(sample) / bucket_divisor();

                index as usize
            };

        // Now index into the appropriate array.

        if sample < 0.0 {
            self.negative[index] += 1;
        } else {
            self.positive[index] += 1;
        }
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

    fn print_negative(&self, printer: &mut dyn Printer, _suppress: &Suppress) {
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

        // Start printing from the highest-index non-zero row.

        let     start_row   = scan / print_roundup();
        let     start_index = start_row * print_roundup();
        let mut rows        = start_row + 1;
        let mut index       = start_index;

        while rows > 0 {
            assert!(index <= self.count - print_roundup());

            let exponent = (index as isize) * bucket_divisor();
            let exponent = exponent - exponent_bias();

            assert!(print_roundup() == 4);    // This format assumes a

            printer.print(&format!("  -2^{:>5}:    {:>14}    {:>14}    {:>14}    {:>14}",
                exponent,
                Printable::commas_u64(self.negative[index + 0]),
                Printable::commas_u64(self.negative[index + 1]),
                Printable::commas_u64(self.negative[index + 2]),
                Printable::commas_u64(self.negative[index + 3])
            ));

            rows -= 1;

            if index >= print_roundup() {
                index -= 4;
            }
        }
    }

    // This helper method prints the positive buckets.

    fn print_positive(&self, printer: &mut dyn Printer, _suppress: &Suppress) {
        let mut last = self.count - 1;

        while last > 0 && self.positive[last] == 0 {
            last -= 1;
        }

        let     stop_index = last;
        let mut i          = 0;

        assert!(print_roundup() == 4);    // This code assumes len() % 4 == 0

        // Skip over rows with entries that are all zero.

        while i <= stop_index {
            if
                self.positive[i    ] == 0
            &&  self.positive[i + 1] == 0
            &&  self.positive[i + 2] == 0
            &&  self.positive[i + 3] == 0 {
                i += 4
            } else {
                break;
            }
        }

        // Print the rows.  Each row has the counts for 4 buckets.

        while i <= stop_index {
            assert!(i <= self.positive.len() - 4);

            let exponent = i as isize * bucket_divisor();
            let exponent = exponent - exponent_bias();

            printer.print(&format!("   2^{:>5}:    {:>14}    {:>14}    {:>14}    {:>14}",
                exponent,
                Printable::commas_u64(self.positive[i]),

                Printable::commas_u64(self.positive[i + 1]),
                Printable::commas_u64(self.positive[i + 2]),
                Printable::commas_u64(self.positive[i + 3])));

            i += 4;
        }
    }

    /// Prints the histogrm.

    pub fn print(&self, printer: &mut dyn Printer) {
        self.print_opts(printer, &self.suppress);
    }

    /// Prints the histogram.  The suppress option is not currently
    /// implemented.

    pub fn print_opts(&self, printer: &mut dyn Printer, suppress: &Suppress) {
        let header =
            format!("  Log Histogram:  ({} NaN, {} infinite)", self.nans, self.infinities);

        printer.print(&header);
        self.print_negative(printer, suppress);
        printer.print("  -----------------------");
        self.print_positive(printer, suppress);
    }

    /// Deletes all data from the histogrm.

    pub fn clear(&mut self) {
        self.negative   = vec![0; self.count];
        self.positive   = vec![0; self.count];
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
        self.print_opts(printer, &self.suppress);
    }

    /// Clear the histogram data.

    fn clear_histogram(&mut self) {
        self.clear()
    }

    /// Convert the pointer to histogram types if possible.

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
    use super::*;

    fn simple_print_test() {
        let     min       = min_exponent();
        let     max       = max_exponent();
        let     suppress  = Suppress { min, max };
        let mut histogram = FloatHistogram::new(suppress);
        let     max_index = max_biased_exponent() / bucket_divisor();

        for i in 0..= max_index {
            histogram.negative[i as usize] = i as u64;
        }

        for i in 0..= max_index {
            histogram.positive[i as usize] = i as u64;
        }

        let     printer_box = stdout_printer();
        let     printer     = &mut *printer_box.lock().unwrap();

        histogram.print(printer);
    }

    #[test]
    fn run_tests() {
        simple_print_test()
    }
}
