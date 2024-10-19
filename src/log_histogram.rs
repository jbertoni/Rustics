//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

/// ## Type
///
/// * LogHistogram
///     * LogHistogram implements a histogram based on a pseudo-log
///       function.  For positive numbers, the pseudo-log is define
///       as the base 2 log of the value, rounded up to an integer.
///       For a negative number n, the pseudo-log is defined as 
///       pseudo-log(-n).
///```
///     use rustics::log_histogram::LogHistogram;
///     use rustics::log_histogram::pseudo_log_index;
///     use rustics::Printer;
///     use rustics::stdout_printer;
///
///     // This is a simple sanity test of the LogHistogram code.  It
///     // hopefull will help you understand the data it produces.
///
///     let mut histogram = LogHistogram::new();
///     let     printer   = &mut stdout_printer();
///
///     let     test      = [ 1, -1, 4, 25, 4109, -4108, -8, -9, -16, -17, 3, 8, 16 ];
///
///     for i in test.iter() {
///         let pseudo_log_index = pseudo_log_index(*i) as usize;
///
///         let expected =
///             if *i < 0 {
///                 histogram.negative[pseudo_log_index] + 1
///             } else {
///                 histogram.positive[pseudo_log_index] + 1
///             };
///
///         histogram.record(*i);
///
///         let actual =
///             if *i < 0 {
///                 histogram.negative[pseudo_log_index]
///             } else {
///                 histogram.positive[pseudo_log_index]
///             };
///
///         assert!(actual == expected);
///      }
///```

// Implement a structure for the pseudo-log histograms.

use super::Printer;
use super::printable::Printable;

// This function returns an array index to record a log value in a
// histogram.  Callers are expected to use two arrays, one for
// positive and one for negative values, so this routine ignores the
// sign of its input.
//
// The algorithm implements a simple log-like function of the
// absolute value of its input.  It is intended only for making
// histograms.
//
// The pseudo-log of a positive integer is its base 2 log rounded
// up to an integer.
//
// The pseudo-log of most negative integers n is defined as -log(-n)
// to give a reasonable histogram structure.  The pseudo-log of
// i64::MIN is defined as 63 for convenience.  This routine always
// returns a non-negative index for an array, so the return value is
// pseudo-log(-n) for negative valeus.  The calling routine handles
// the negation by using separate arrays for positive and negative
// pseudo-log values.
//
// In addition to the above notes, pseudo-log(0) is defined as 0.
//

pub fn pseudo_log_index(value: i64) -> usize {
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
                Printable::commas_u64(self.negative[i - 3]),
                Printable::commas_u64(self.negative[i - 2]),
                Printable::commas_u64(self.negative[i - 1]),
                Printable::commas_u64(self.negative[i])
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
                Printable::commas_u64(self.positive[i]),
                Printable::commas_u64(self.positive[i + 1]),
                Printable::commas_u64(self.positive[i + 2]),
                Printable::commas_u64(self.positive[i + 3])));

            i += 4;
        }
    }

    // Find the most common "log" bucket

    pub fn log_mode(&self) -> isize {
        let mut mode = 0;
        let mut max  = 0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestPrinter;

    pub fn test_log_histogram() {
        let mut histogram = LogHistogram::new();
        let     printer   = &mut TestPrinter::new(&"Test Output");
        let     test      = [ 1, -1, 4, 25, 4109, -4108, -8, -9, -16, -17, 3, 8, 16 ];

        for i in test.iter() {
            let pseudo_log_index = pseudo_log_index(*i) as usize;

            let expected =
                if *i < 0 {
                    histogram.negative[pseudo_log_index] + 1
                } else {
                    histogram.positive[pseudo_log_index] + 1
                };

            histogram.record(*i);

            let actual =
                if *i < 0 {
                    histogram.negative[pseudo_log_index]
                } else {
                    histogram.positive[pseudo_log_index]
                };

            assert!(actual == expected);
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

    #[test]
    fn run_tests() {
        test_log_histogram();
        test_pseudo_log();
    }
}
