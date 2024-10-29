//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::cmp::min;
use super::Histogram;
use super::FloatHistogramBox;
use super::LogHistogramBox;
use super::Printer;
use super::to_exponent;
use super::min_exponent;
use super::max_exponent;

pub type SuppressOption  = Option<Suppress>;

pub struct Suppress {
    pub min:  isize,
    pub max:  isize,
}

pub struct FloatHistogram {
    negative:   Vec<u64>,
    positive:   Vec<u64>,
    nans:       usize,
    infinities: usize,
    suppress:   Suppress,
}

fn bucket_divisor() -> isize {
    16
}

fn buckets() -> isize {
    (max_exponent() - min_exponent()) / bucket_divisor()
}

impl FloatHistogram {
    pub fn new(suppress: Suppress) -> FloatHistogram {
        let count      = buckets() as usize;
        let negative   = vec![0; count];
        let positive   = vec![0; count];
        let nans       = 0;
        let infinities = 0;

        FloatHistogram { negative, positive, nans, infinities, suppress }
    }

    pub fn record(&mut self, sample: f64) {
        if sample.is_nan() {
            self.nans += 1;
            return;
        }

        if sample.is_infinite() {
            self.infinities += 1;

            if sample < 0.0 {
                let index = self.negative.len() - 1;

                self.negative[index] += 1;
            } else {
                let index = self.positive.len() - 1;

                self.positive[index] += 1;
            }

            return;
        }

        let exponent = to_exponent(sample) + min_exponent(); 
        let exponent = exponent / bucket_divisor();
        let exponent = min(exponent, buckets());
        let exponent = exponent as usize;

        if sample < 0.0 {
            self.negative[exponent] += 1;
        } else {
            self.positive[exponent] += 1;
        }
    }

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

        mode
    }

    fn print_negative(&self, _printer: &mut dyn Printer, _suppress: &Suppress) {
        // TODO
    }

    fn print_positive(&self, _printer: &mut dyn Printer, _suppress: &Suppress) {
        // TODO
    }

    pub fn print(&self, printer: &mut dyn Printer) {
        self.print_opts(printer, &self.suppress);
    }

    pub fn print_opts(&self, printer: &mut dyn Printer, suppress: &Suppress) {
        let header =
            format!("  Log Histogram:  ({} NaN, {} infinite)", self.nans, self.infinities);

        printer.print(&header);
        self.print_negative(printer, suppress);
        printer.print("  -----------------------");
        self.print_positive(printer, suppress);
    }

    pub fn clear(&mut self) {
        let count    = buckets() as usize;

        self.negative = vec![0; count];
        self.positive = vec![0; count];

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
