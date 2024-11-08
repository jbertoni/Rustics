//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//! ## Type
//!
//! * IntegerWindow
//!     * IntegerWindow maintains a set consisting of the last n samples
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
//!    use rustics::integer_window::IntegerWindow;
//!
//!    // Create an instance to record packet sizes.  The default for
//!    // printing output is stdout, which we'll assume is fine for this
//!    // example, so None works for the print options.  See the
//!    // RunningInteger comments for an example of how to set print
//!    // options.
//!    //
//!    // Assume that retaining 1000 samples is fine for our hypothetical
//!    // application.
//!
//!    let window_size = 1000;
//!
//!    let mut packet_sizes =
//!        IntegerWindow::new("Packet Sizes", window_size, &None);
//!
//!    // Record some hypothetical packet sizes.  Let's fill the window.
//!
//!    for i in 1..=window_size {
//!       packet_sizes.record_i64(i as i64);
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
//!    assert!(packet_sizes.mean() == mean);
//!
//!    // Let's record more samples.  The count only includes the last
//!    // "window_size" samples, so it should be constant now.
//!
//!    for i in 1..=window_size / 2 {
//!       packet_sizes.record_i64(i as i64);
//!       assert!(packet_sizes.count() == window_size as u64);
//!    }
//!```

use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;

use super::Rustics;
use super::Printer;
use super::PrinterBox;
use super::ExportStats;
use super::PrinterOption;
use super::PrintOption;
use super::Units;
use super::TimerBox;
use super::Histogram;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use crate::printable::Printable;
use crate::log_histogram::LogHistogram;
use super::printer_mut;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::sum::kbk_sum;
use super::sum::kbk_sum_sort;
use super::parse_print_opts;

/// An IntegerWindow instance collects integer data samples into
/// a fixed-size window. It also maintains a histogram based on
/// all the samples seen.
///
/// See the module documentation for sample code.

#[derive(Clone)]
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

    log_histogram:  LogHistogramBox,

    printer:        PrinterBox,
    units:          Units,
}

// The Crunched structure contains all the data needed to
// compute the summary statistics that we need to print.

/// The Crunched struct is used to pass summary data from a
/// statistics set to printing functions.  It is used internally
/// and is intended for use by code implementing data types,
/// not users collecting data.

#[derive(Clone, Copy, Default)]
pub struct Crunched {
    pub mean:       f64,
    pub sum:        f64,
    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,
}

impl Crunched {
    pub fn zero() -> Crunched {
        let mean     = 0.0;
        let sum      = 0.0;
        let moment_2 = 0.0;
        let moment_3 = 0.0;
        let moment_4 = 0.0;

        Crunched { mean, sum, moment_2, moment_3, moment_4 }
    }
}

impl IntegerWindow {
    pub fn new(name: &str, window_size: usize, print_opts: &PrintOption) -> IntegerWindow {
        if window_size == 0 {
            panic!("The window size is zero.");
        }

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
        let log_histogram = LogHistogram::new();
        let log_histogram = Rc::from(RefCell::new(log_histogram));

        let (printer, title, units, _histo_opts) = parse_print_opts(print_opts, &name);

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
            sum += *sample as f64;
        }

        sum
    }

    /// Gather the statistics and compute summary statistics
    /// for the current samples in the window.

    pub fn crunch(&self) -> Crunched {
        if self.vector.is_empty() {
            return Crunched::zero();
        }

        let mut samples = Vec::new();

        for value in self.vector.iter() {
            samples.push(*value as f64)
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

            vec_2.push(square           );
            vec_3.push(square * distance);
            vec_4.push(square * square  );
        }

        // Use kbk_sum to try to get more precision.  The samples
        // vector was sorted by kbk_sum_sort, so these vectors are sorted
        // already.

        let moment_2 = kbk_sum(&vec_2);
        let moment_3 = kbk_sum(&vec_3);
        let moment_4 = kbk_sum(&vec_4);

        Crunched { mean, sum, moment_2, moment_3, moment_4 }
    }

    fn compute_min(&self) -> i64 {
        match self.vector.iter().min() {
            Some(min) => *min,
            None      => 0,
        }
    }

    fn compute_max(&self) -> i64 {
        match self.vector.iter().max() {
            Some(max) => *max,
            None      => 0,
        }
    }

    pub fn get_printable(&self) -> Printable {
        let n          = self.vector.len() as u64;
        let nans       = 0;
        let infinities = 0;
        let min_i64    = self.compute_min();
        let max_i64    = self.compute_max();
        let min_f64    = f64::MIN;
        let max_f64    = f64::MAX;
        let log_mode   = self.log_histogram.borrow().log_mode() as i64;
        let mode_value = 0.0;
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

    #[cfg(test)]
    pub fn analyze(&self) -> AnalyzeData {
        let mut copy         = Vec::new();
        let mut squared      = Vec::new();
        let mut cubed        = Vec::new();
        let mut quadded      = Vec::new();

        for sample in &self.vector {
            let sample = *sample as f64;

            copy   .push(sample        );
            squared.push(sample.powi(2));
            cubed  .push(sample.powi(3));
            quadded.push(sample.powi(4));
        }

        let sum      = kbk_sum_sort(&mut copy   );
        let squares  = kbk_sum_sort(&mut squared);
        let cubes    = kbk_sum_sort(&mut cubed  );
        let quads    = kbk_sum_sort(&mut quadded);

        let n        = self.vector.len() as f64;
        let mean     = sum / n;

        let mut moment_2_vec = Vec::new();
        let mut moment_3_vec = Vec::new();
        let mut moment_4_vec = Vec::new();

        for sample in copy.iter() {
            moment_2_vec.push((*sample - mean).powi(2));
            moment_3_vec.push((*sample - mean).powi(3));
            moment_4_vec.push((*sample - mean).powi(4));
        }

        let moment_2 = kbk_sum(&moment_2_vec);
        let moment_3 = kbk_sum(&moment_3_vec);
        let moment_4 = kbk_sum(&moment_4_vec);

        let min_i64 = self.compute_min();
        let max_i64 = self.compute_max();
        let min_f64 = 0.0;
        let max_f64 = 0.0;

        AnalyzeData {
            n,          sum,        squares,    cubes,      quads,
            moment_2,   moment_3,   moment_4,
            min_i64,    max_i64,    min_f64,    max_f64
        }
    }
}

#[cfg(test)]
pub struct AnalyzeData {
    pub n:          f64,
    pub sum:        f64,
    pub squares:    f64,
    pub cubes:      f64,
    pub quads:      f64,

    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,

    pub min_i64:    i64,
    pub max_i64:    i64,
    pub min_f64:    f64,
    pub max_f64:    f64,
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

        self.log_histogram.borrow_mut().record(sample);
        self.stats_valid = false;
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::IntegerWindow:  f64 samples are not permitted.");
    }

    fn record_event(&mut self) {
        panic!("Rustics::IntegerWindow:  event samples are not permitted.");
    }

    fn record_event_report(&mut self) -> i64 {
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
        self.log_histogram.borrow().log_mode()
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

        if self.stats_valid {
            compute_skewness(count, self.moment_2, self.moment_3)
        } else {
            let crunched = self.crunch();

            compute_skewness(count, crunched.moment_2, crunched.moment_3)
        }
    }

    fn kurtosis(&self) -> f64 {
        let count = self.vector.len() as u64;

        if self.stats_valid {
            compute_kurtosis(count, self.moment_2, self.moment_3)
        } else {
            let crunched = self.crunch();

            compute_kurtosis(count, crunched.moment_2, crunched.moment_4)
        }
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn float_extremes(&self) -> bool {
        false
    }

    fn min_f64(&self) -> f64 {
        panic!("IntegerWindow:: min_f64 is not supported.");
    }

    fn max_f64(&self) -> f64 {
        panic!("IntegerWindow:: max_f64 is not supported.");
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
        self.log_histogram.borrow_mut().clear();

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
        let printer   = printer_mut!(printer_box);

        printer.print(title);
        printable.print_common_i64(printer);
        printable.print_common_float(printer);
        self.log_histogram.borrow().print(printer);
        printer.print("");
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

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        Some(self.log_histogram.clone())
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        None
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
        let log_histogram   = Some(self.log_histogram.clone());
        let float_histogram = None;

        ExportStats { printable, log_histogram, float_histogram }
    }
}

impl Histogram for IntegerWindow {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.log_histogram.borrow().print(printer);
    }

    fn clear_histogram(&mut self) {
        self.log_histogram.borrow_mut().clear();
    }

    fn to_log_histogram(&self) -> Option<LogHistogramBox> {
        Some(self.log_histogram.clone())
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::PrintOpts;
    use crate::log_histogram::pseudo_log_index;
    use crate::tests::continuing_box;
    use crate::running_integer::RunningInteger;
    use crate::tests::check_printer_box;
    use crate::tests::bytes;

    pub fn test_simple_integer_window() {
        let     window_size = 100;
        let mut stats       = IntegerWindow::new(&"Test Statistics", window_size, &None);

        assert!(stats.class() == "integer");
        assert!( stats.int_extremes  ());
        assert!(!stats.float_extremes());

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

        // Check precompute().

        stats.precompute();
        assert!(stats.mean() == sample as f64);

        stats.precompute();
        assert!(stats.mean() == sample as f64);

        stats.clear();

        assert!(stats.min_i64() == 0);
        assert!(stats.max_i64() == 0);

        stats.record_i64(-1);
        assert!(stats.min_i64() == -1);
        assert!(stats.max_i64() == -1);

        stats.record_i64(1);
        assert!(stats.min_i64() == -1);
        assert!(stats.max_i64() ==  1);

        assert!(stats.count() == 2  );
        assert!(stats.mean()  == 0.0);

        stats.clear();
        let sample = 4;

        for _i in 0..10 {
            stats.record_i64(sample);
        }

        let sample = sample as f64;

        assert!(stats.standard_deviation() == 0.0);

        assert!(stats.mean()     == sample);
        assert!(stats.variance() == 0.0   );
        assert!(stats.skewness() == 0.0   );
        assert!(stats.kurtosis() == 0.0   );
    }

    fn test_equality() {
        let stats_1 = IntegerWindow::new (&"Equal 1", 10, &None);
        let stats_2 = IntegerWindow::new (&"Equal 2", 10, &None);
        let stats_3 = RunningInteger::new(&"Equal 3",     &None);

        assert!( stats_1.equals(&stats_1));
        assert!(!stats_1.equals(&stats_2));
        assert!(!stats_1.equals(&stats_3));
    }

    #[test]
    #[should_panic]
    fn test_record_f64() {
        let mut stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        stats.record_f64(1.0);
    }

    #[test]
    #[should_panic]
    fn test_record_event() {
        let mut stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        stats.record_event();
    }

    #[test]
    #[should_panic]
    fn test_record_event_report() {
        let mut stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        let _ = stats.record_event_report();
    }

    #[test]
    #[should_panic]
    fn test_record_time() {
        let mut stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        stats.record_time(1);
    }

    #[test]
    #[should_panic]
    fn test_record_interval() {
        let mut timer  = continuing_box();
        let mut stats  = IntegerWindow::new(&"Test Statistics", 20, &None);

        stats.record_interval(&mut timer);
    }

    #[test]
    #[should_panic]
    fn test_min_f64() {
        let stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        let _  = stats.min_f64();
    }

    #[test]
    #[should_panic]
    fn test_max_f64() {
        let stats = IntegerWindow::new(&"Test Statistics", 20, &None);

        let _  = stats.max_f64();
    }

    #[test]
    #[should_panic]
    fn test_zero_size() {
        let _stats = IntegerWindow::new(&"Test Statistics", 0, &None);
    }

    fn test_histogram() {
        let     size  = 100;
        let mut stats = IntegerWindow::new(&"Test Statistics", size, &None);

        for i in 1..=size {
            stats.record_i64(i as i64);
        }

        {
            let histogram = stats.to_log_histogram().unwrap();
            let histogram = histogram.borrow();

            let mut sum = 0;

            for sample in histogram.positive.iter() {
                sum += *sample;
            }

            assert!(sum == size as u64);
        }
        {
            stats.clear_histogram();

            let histogram = stats.to_log_histogram().unwrap();
            let histogram = histogram.borrow();

            let mut sum = 0;

            for sample in histogram.positive.iter() {
                sum += *sample;
            }

            assert!(sum == 0);
        }
    }

    #[test]
    #[should_panic]
    fn test_float_histogram() {
        let stats = IntegerWindow::new(&"Test Statistics", 100, &None);
        let _     = stats.to_float_histogram().unwrap();
    }

    fn test_print_output() {
        let expected =
            [
                "Test Statistics",
                "    Count               1,000 ",
                "    Minumum                 1 byte",
                "    Maximum             1,000 bytes",
                "    Log Mode               10 ",
                "    Mode Value          1,024 bytes",
                "    Mean             +5.00500 e+2 bytes",
                "    Std Dev          +2.88819 e+2 bytes",
                "    Variance         +8.34166 e+4 ",
                "    Skewness         +0.00000 e+0 ",
                "    Kurtosis         -1.20000 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:               128               256               488                 0",
                ""
            ];

        let     printer    = Some(check_printer_box(&expected, false, false));
        let     title      = None;
        let     units      = bytes();
        let     histo_opts = None;
        let     print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let     name       = "Test Statistics";
        let     samples    = 1000;
        let mut stats      = IntegerWindow::new(&name, samples, &print_opts);

        for i in 1..=samples {
            stats.record_i64(i as i64);
        }

        stats.print();
    }

    #[test]
    fn run_tests() {
        test_simple_integer_window();
        test_equality();
        test_histogram();
        test_print_output();
    }
}
