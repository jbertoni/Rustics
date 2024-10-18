//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::any::Any;

use super::Rustics;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::TimerBox;
use super::Histogram;
use crate::printable::Printable;
use crate::log_histogram::LogHistogram;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::stdout_printer;

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

#[derive(Default)]
pub struct Crunched {
    pub mean:       f64,
    pub sum:        f64,
    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,
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
    pub fn new(name_in: &str, window_size: usize, printer: PrinterOption) -> IntegerWindow {
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

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

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

    pub fn crunch(&self) -> Crunched {
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
                printer
            } else {
                self.printer.clone()
            };

        let title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

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
        let printer   = &mut *printer_box.lock().unwrap();

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

    fn histogram(&self) -> LogHistogram {
        self.log_histogram.clone()
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

    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.log_histogram.print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_histogram::pseudo_log_index;

    pub fn test_simple_integer_window() {
        let window_size = 100;
        let mut stats = IntegerWindow::new(&"Test Statistics", window_size, None);

        assert!(stats.class() == "integer");

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

    #[test]
    fn run_tests() {
        test_simple_integer_window();
    }
}
