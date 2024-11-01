//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! ## Type
//!
//! * FloatWindow
//!     * FloatWindow maintains a set consisting of the last n samples
//!       recorded into it.
//!
//!     * This type also maintains a log histogram that contains counts
//!       of all events seen, not just the window of n samples.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use rustics::Rustics;
//!    use rustics::float_window::FloatWindow;
//!
//!    // Create an instance to record packet sizes in kbytes.  The
//!    // default for printing output is stdout, which we'll assume
//!    // is fine for this example, so None works for the printer.
//!    //
//!    // Assume that retaining 1000 samples is fine for our hypothetical
//!    // application.
//!
//!    let window_size = 1000;
//!
//!    let mut packet_sizes =
//!        FloatWindow::new("Packet Sizes", window_size, &None, &None);
//!
//!    // Record some hypothetical packet sizes.  Let's fill the window.
//!
//!    for i in 1..=window_size {
//!       packet_sizes.record_f64(i as f64);
//!       assert!(packet_sizes.count() == i as u64);
//!    }
//!
//!    // Print our statistics.
//!
//!    packet_sizes.print();
//!
//!    // We should have seen "window_size" events.
//!
//!    assert!(packet_sizes.count() == window_size as u64);
//!
//!    // Compute the expected mean.  We need the sum of all the packet
//!    // sizes:
//!    //     1 + 2 + ... + n
//!    // The formula is:
//!    //     n * (n + 1) / 2
//!
//!    let float_count = window_size as f64;
//!    let float_sum   = float_count * (float_count + 1.0) / 2.0;
//!    let mean        = float_sum / float_count;
//!
//!    assert!(packet_sizes.mean()    == mean       );
//!    assert!(packet_sizes.min_f64() == 1.0        );
//!    assert!(packet_sizes.max_f64() == float_count);
//!
//!    // Let's record more samples.  The count only includes the last
//!    // "window_size" samples, so it should be constant now.
//!
//!    for i in 1..=window_size / 2 {
//!       packet_sizes.record_f64(i as f64);
//!       assert!(packet_sizes.count() == window_size as u64);
//!    }
//!```

use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;

use super::Rustics;
use super::ExportStats;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::Units;
use super::Histogram;
use super::LogHistogramBox;
use super::float_histogram::FloatHistogram;
use super::FloatHistogramBox;
use super::integer_window::Crunched;
use super::TimerBox;
use super::printable::Printable;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::sum::kbk_sum;
use super::sum::kbk_sum_sort;
use super::parse_print_opts;
use super::float_histogram::HistoOpts;
use super::float_histogram::HistoOption;

/// An FloatWindow instance collects integer data samples into
/// a fixed-size window. It also maintains a histogram based on
/// all the samples seen.
///
/// See the module documentation for sample code.

#[derive(Clone)]
pub struct FloatWindow {
    name:           String,
    title:          String,
    window_size:    usize,
    vector:         Vec<f64>,
    id:             usize,

    // These fields must be zeroed or reset in clear():

    index:          usize,
    stats_valid:    bool,

    //  These fields are computed when stats_valid is false and
    // the user requests statistical information.

    mean:       f64,
    sum:        f64,
    moment_2:   f64,
    moment_3:   f64,
    moment_4:   f64,

    histogram:  FloatHistogramBox,

    printer:    PrinterBox,
    units:      Units,
}

impl FloatWindow {
    pub fn new(name: &str, window_size: usize, print_opts: &PrintOption, histo_opts: &HistoOption)
            -> FloatWindow {
        if window_size == 0 {
            panic!("The window size is zero.");
        }

        let histo_opts =
            if let Some(histo_opts) = histo_opts {
                histo_opts
            } else {
                &HistoOpts::default()
            };

        let name          = String::from(name);
        let id            = usize::MAX;
        let vector        = Vec::with_capacity(window_size);
        let index         = 0;
        let stats_valid   = false;
        let mean          = 0.0;
        let sum           = 0.0;
        let moment_2      = 0.0;
        let moment_3      = 0.0;
        let moment_4      = 0.0;
        let histogram     = FloatHistogram::new(histo_opts);
        let histogram     = Rc::from(RefCell::new(histogram));

        let (printer, title, units) = parse_print_opts(print_opts, &name);

        FloatWindow {
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
            histogram,
            printer,
            units
        }
    }

    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    fn sum(&self) -> f64 {
        let mut sum = 0.0;

        for sample in self.vector.iter() {
            sum += *sample;
        }

        sum
    }

    /// Gather the statistics and compute summary statistics
    /// for the current samples in the window.

    pub fn crunch(&self) -> Crunched {
        if self.vector.is_empty() {
            return Crunched::new();
        }

        let mut samples = Vec::new();

        for value in self.vector.iter() {
            samples.push(*value);
        }

        let sum  = kbk_sum_sort(&mut samples);
        let mean =  sum / self.vector.len() as f64;

        // Create the vectors of the addends for the moments about
        // the mean.

        let mut vec_2 = Vec::new();
        let mut vec_3 = Vec::new();
        let mut vec_4 = Vec::new();

        // Now fill the vectors with addends.

        for sample in samples.iter() {
            let distance = *sample - mean;
            let square   = distance * distance;

            vec_2.push(square);
            vec_3.push(distance * square);
            vec_4.push(square   * square);
        }

        // Use kbk_sum_no_sort to try to get more precision.  The
        // samples vector was sorted by kbk_sum, so these vectors
        // are sorted already.

        let moment_2 = kbk_sum(&vec_2);
        let moment_3 = kbk_sum(&vec_3);
        let moment_4 = kbk_sum(&vec_4);

        Crunched { mean, sum, moment_2, moment_3, moment_4 }
    }

    fn compute_min(&self) -> f64 {
        if self.vector.is_empty() {
            return 0.0;
        }

        let mut min = self.vector[0];

        for i in 1..self.vector.len() {
            if self.vector[i] < min {
                min = self.vector[i];
            }
        }

        min
    }

    fn compute_max(&self) -> f64 {
        if self.vector.is_empty() {
            return 0.0;
        }

        let mut max = self.vector[0];

        for i in 1..self.vector.len() {
            if self.vector[i] > max {
                max = self.vector[i];
            }
        }

        max
    }

    pub fn get_printable(&self) -> Printable {
        let n          = self.vector.len() as u64;
        let nans       = 0;
        let infinities = 0;
        let min_i64    = i64::MIN;
        let max_i64    = i64::MAX;
        let min_f64    = self.compute_min();
        let max_f64    = self.compute_max();
        let log_mode   = 0;
        let mode_value = self.histogram.borrow().mode_value();
        let units      = self.units.clone();

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

        Printable {
            n,         nans,  infinities,  min_i64,   max_i64,   min_f64,    max_f64,
            log_mode,  mean,  variance,    skewness,  kurtosis,  mode_value,  units
        }
    }
}

impl Rustics for FloatWindow {
    fn record_f64(&mut self, sample: f64) {
        if self.vector.len() == self.window_size {
            self.vector[self.index] = sample;
            self.index += 1;

            if self.index >= self.window_size {
                self.index = 0;
            }
        } else {
            self.vector.push(sample);
        }

        self.histogram.borrow_mut().record(sample);
        self.stats_valid = false;
    }

    fn record_i64(&mut self, _sample: i64) {
        panic!("Rustics::FloatWindow:  i64 samples are not permitted.");
    }

    fn record_event(&mut self) {
        panic!("Rustics::FloatWindow:  event samples are not permitted.");
    }

    fn record_event_report(&mut self) -> i64 {
        panic!("Rustics::FloatWindow:  event samples are not permitted.");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Rustics::FloatWindow:  time samples are not permitted.");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Rustics::FloatWindow:  time intervals are not permitted.");
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
        panic!("FloatWindow::log_mode:  log_mode not supported");
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

        if self.stats_valid {
            compute_variance(count, self.moment_2)
        } else {
            let crunched = self.crunch();

            compute_variance(count, crunched.moment_2)
        }
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
        false
    }

    fn float_extremes(&self) -> bool {
        true
    }

    fn min_f64(&self) -> f64 {
        self.compute_min()
    }

    fn max_f64(&self) -> f64 {
        self.compute_max()
    }

    fn min_i64(&self) -> i64 {
        panic!("FloatWindow::min_i64:  not supported");
    }

    fn max_i64(&self) -> i64 {
        panic!("FloatWindow::max_i64:  not supported");
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
        self.histogram.borrow_mut().clear();

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

        let printable = self.get_printable();
        let printer   = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_f64(printer);
        printable.print_common_float(printer);
        self.histogram.borrow().print(printer);
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<FloatWindow>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        None
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        Some(self.histogram.clone())
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

    fn export_stats(&self) -> ExportStats {
        let printable       = self.get_printable();
        let log_histogram   = None;
        let float_histogram = Some(self.histogram.clone());

        ExportStats { printable, log_histogram, float_histogram }
    }
}

impl Histogram for FloatWindow {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.histogram.borrow().print(printer);
    }

    fn clear_histogram(&mut self) {
        self.histogram.borrow_mut().clear();
    }

    fn to_log_histogram(&self) -> Option<LogHistogramBox> {
        None
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        Some(self.histogram.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_simple_float_window() {
        let     window_size = 100;

        let mut stats =
            FloatWindow::new(&"Test Statistics", window_size, &None, &None);

        assert!(stats.class() == "integer");
        assert!(!stats.int_extremes  ());
        assert!( stats.float_extremes());

        for sample in -256..512 {
            stats.record_f64(sample as f64);
        }

        assert!(stats.histogram.borrow().mode_value() == 2.0_f64);

        stats.print();
        let sample = 100;

        for _i in 0..2 * window_size {
            stats.record_f64(sample as f64);
        }

        stats.print();
        assert!(stats.mean() == sample as f64);
        assert!(stats.histogram.borrow().mode_value() == 2.0_f64);

        stats.clear();

        assert!(stats.min_f64() == 0.0);
        assert!(stats.max_f64() == 0.0);

        stats.record_f64(-1.0);
        assert!(stats.min_f64() == -1.0);
        assert!(stats.max_f64() == -1.0);

        stats.record_f64(1.0);
        assert!(stats.min_f64() == -1.0);
        assert!(stats.max_f64() ==  1.0);

        assert!(stats.count() == 2  );
        assert!(stats.mean()  == 0.0);

        stats.clear();
        let sample = 4.0;

        for _i in 0..10 {
            stats.record_f64(sample);
        }

        assert!(stats.standard_deviation() == 0.0);

        assert!(stats.mean()     == sample);
        assert!(stats.variance() == 0.0   );
        assert!(stats.skewness() == 0.0   );
        assert!(stats.kurtosis() == 0.0   );
    }

    #[test]
    fn run_tests() {
        test_simple_float_window();
    }
}