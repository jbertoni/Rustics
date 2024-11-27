//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//!
//! ## Type
//! * RunningFloat
//!   * RunningFloat provides statistical summaries of samples of type
//!     f64.
//!
//!   * This includes a very coarse log histogram similar to the one
//!     that is provided for i64 data.
//!
//! ## Example
//!```
//!     use rustics::Rustics;
//!     use rustics::PrintOpts;
//!     use rustics::float_histogram::HistoOpts;
//!     use rustics::ExportStats;
//!     use rustics::printable::Printable;
//!     use rustics::running_float::RunningFloat;
//!
//!     // Accept the default print options except for the histogram
//!     // option.  See the RunningInteger comments for an example of
//!     // how to set the other print options.
//!
//!     // Create a HistoOpts for new().
//!
//!     let merge_min    = 0;  // not implemented yet
//!     let merge_max    = 0;  // not implemented yet
//!     let no_zero_rows = true;
//!
//!     let histo_opts = HistoOpts { merge_min, merge_max, no_zero_rows };
//!     let histo_opts = Some(histo_opts);
//!     let printer    = None;
//!     let title      = None;
//!     let units      = None;
//!     let print_opts = PrintOpts { printer, title, units, histo_opts };
//!     let print_opts = Some(print_opts);
//!
//!     let mut float = RunningFloat::new("Test Statistic", &print_opts);
//!     let     end   = 1000;
//!
//!     // Record the integers from 1 to "end".
//!
//!     for i in 1..=end {
//!         float.record_f64(i as f64);
//!     }
//!
//!     // Print our data.
//!
//!     float.print();
//!
//!    // Compute the expected mean.  We need the sum of
//!    //     1 + 2 + ... + n
//!    // which is
//!    //     n * (n + 1) / 2.
//!
//!     let float_end = end as f64;
//!     let sum       = (float_end * (float_end + 1.0)) / 2.0;
//!     let mean      = sum / float_end;
//!
//!     assert!(float.count()   == end as u64);
//!     assert!(float.mean()    == mean      );
//!     assert!(float.min_f64() == 1.0       );
//!     assert!(float.max_f64() == float_end );
//!
//!     // The code should keep count of NaNs and non-finite
//!     // values, but not record them for the mean, etc.
//!
//!     float.record_f64(f64::INFINITY);
//!     float.record_f64(f64::NEG_INFINITY);
//!     float.record_f64(f64::NAN);
//!
//!     float.print();
//!
//!     // NaN and non-finite values shouldn't be counted
//!     // as samples.
//!
//!     assert!(float.mean()    == mean      );
//!     assert!(float.count()   == end as u64);
//!
//!     // Check that the non-finite values were counted in their
//!     // special counters.  Test export_stats and the more direct
//!     // methods of getting those counts.
//!
//!     let stats = float.export_stats();
//!
//!     assert!(stats.printable.n          == end);
//!     assert!(stats.printable.nans       == 1);
//!     assert!(stats.printable.infinities == 2);
//!
//!     assert!(float.nans()               == 1);
//!     assert!(float.infinities()         == 2);
//!
//!     // Print all the summary information.
//!
//!     float.print();
//!```

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::Rustics;
use super::Histogram;
use super::Printer;
use super::hier::HierExporter;
use super::LogHistogramBox;
use super::TimerBox;
use super::ExportStats;
use super::Printable;
use super::PrintOption;
use super::PrinterOption;
use super::PrinterBox;
use super::Units;
use super::parse_print_opts;
use super::compute_variance;
use super::EstimateData;
use super::estimate_moment_3;
use super::compute_skewness;
use super::compute_kurtosis;
use super::FloatHistogram;
use super::FloatHistogramBox;
use super::printer_mut;
use super::min_f64;
use super::max_f64;
use super::merge::Export;
use super::merge::sum_running;

// FloatExporter instances are used to export statistics from a
// RunningFloat instance so that multiple RunningFloat instances can
// be summed.  This is used by FloatHier to allow the Hier code to use
// RunningFloat instances.

/// FloatExport is used by a Hier instance to make summations of
/// multiple RunningFloat instances.

#[derive(Clone, Default)]
pub struct FloatExporter {
    addends: Vec<Export>,
}

/// FloatExporter creates a sum of RunningFloat instances.

impl FloatExporter {
    /// Creates a new FloatExporter instance.

    pub fn new() -> FloatExporter {
        let addends = Vec::new();

        FloatExporter { addends }
    }

    /// Pushes a Rustics instance onto the list of instances to
    /// be summed.

    pub fn push(&mut self, addend: Export) {
        self.addends.push(addend);
    }

    /// Makes a Rustics instance based on the given exports.

    pub fn make_member(&mut self, name: &str, print_opts: &PrintOption) -> RunningFloat {
        let title   = name;
        let sum     = sum_running(&self.addends);

        RunningFloat::new_from_exporter(name, title, print_opts, sum)
    }

    pub fn count(&self) -> usize {
        self.addends.len()
    }
}

// The Hier code uses this trait to do summation of statistics.
//
// We just need downcasting capabilities since all the work
// is implementation-specific.

impl HierExporter for FloatExporter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// This type implements a simple set of statistics for a
/// sequence of f64 values.

pub struct RunningFloat {
    name:       String,
    id:         usize,
    count:      u64,
    nans:       u64,
    infinities: u64,
    mean:       f64,
    moment_2:   f64,
    cubes:      f64,
    moment_4:   f64,
    min:        f64,
    max:        f64,
    title:      String,
    units:      Units,
    histogram:  FloatHistogramBox,
    printer:    PrinterBox,
}

impl RunningFloat {
    /// Constructs a new instance.  print_opts configures the
    /// output of print functions.  "None" will accept the defaults,
    /// which sends the output to stdout.

    pub fn new(name: &str, print_opts: &PrintOption) -> RunningFloat {
        let (printer, title, units, _histo_opts) = parse_print_opts(print_opts, name);

        let name        = name.to_string();
        let id          = usize::MAX;
        let count       = 0;
        let nans        = 0;
        let infinities  = 0;
        let min         = f64::MAX;
        let max         = f64::MIN;
        let mean        = 0.0;
        let moment_2    = 0.0;
        let cubes       = 0.0;
        let moment_4    = 0.0;
        let histogram   = FloatHistogram::new(print_opts);
        let histogram   = Rc::from(RefCell::new(histogram));

        RunningFloat {
            name,      id,        count,    nans,   infinities,  mean,   moment_2,
            cubes,     moment_4,  max,      min,    title,       units,  printer,
            histogram
        }
    }

    /// Creates a new instancer from the data in an exporter.  This is
    /// used internally by the Hier code.

    pub fn new_from_exporter(name: &str, title: &str, print_opts: &PrintOption, import: Export)
            -> RunningFloat {
        let (printer, _title, units, _histo_opts) = parse_print_opts(print_opts, name);

        let name       = String::from(name);
        let title      = title.to_string();
        let id         = usize::MAX;
        let count      = import.count;
        let nans       = import.nans;
        let infinities = import.infinities;
        let mean       = import.mean;
        let moment_2   = import.moment_2;
        let cubes      = import.cubes;
        let moment_4   = import.moment_4;
        let min        = import.min_f64;
        let max        = import.max_f64;
        let histogram  = import.float_histogram.unwrap();

        RunningFloat {
            name,       title,      id,
            count,      mean,       moment_2,
            cubes,      moment_4,   histogram,
            min,        max,        printer,
            units,      nans,       infinities
        }
    }

    fn get_printable(&self) -> Printable {
        let n           = self.count;
        let nans        = self.nans;
        let infinities  = self.infinities;
        let min_i64     = i64::MIN;
        let max_i64     = i64::MAX;
        let min_f64     = self.min;
        let max_f64     = self.max;
        let mode_value  = self.histogram.borrow().mode_value();
        let log_mode    = 0;
        let mean        = self.mean;
        let variance    = self.variance();
        let skewness    = self.skewness();
        let kurtosis    = self.kurtosis();
        let units       = self.units.clone();

        Printable {
            n,         nans,  infinities,  min_i64,   max_i64,   min_f64,  max_f64,
            log_mode,  mean,  variance,    skewness,  kurtosis,  units,    mode_value
        }
    }

    pub fn nans(&self) -> u64 {
        self.nans
    }

    pub fn infinities(&self) -> u64 {
        self.infinities
    }

    /// Exports all the statistics kept for a given instance.
    /// This data is used to create a sum of multiple instances.

    pub fn export_data(&self) -> Export {
        let count           = self.count;
        let nans            = self.nans;
        let infinities      = self.infinities;
        let mean            = self.mean;
        let moment_2        = self.moment_2;
        let cubes           = self.cubes;
        let moment_4        = self.moment_4;
        let float_histogram = Some(self.histogram.clone());
        let log_histogram   = None;
        let min_i64         = 0;
        let max_i64         = 0;
        let min_f64         = self.min;
        let max_f64         = self.max;

        Export {
            count,      nans,       infinities,
            mean,       moment_2,   cubes,
            moment_4,   min_i64,    max_i64,
            min_f64,    max_f64,
            float_histogram,  log_histogram
        }
    }

    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }
}

impl Rustics for RunningFloat {
    fn record_i64(&mut self, _sample: i64) {
        panic!("RunningFloat::record_i64: not supported");
    }

    /// Record an f64 sample.  NaN and infinite values are counted
    /// but otherwise ignored.

    fn record_f64(&mut self, sample: f64) {
        // Ignore NaNs for now.

        if sample.is_nan() {
            self.nans += 1;
            return;
        }

        // Ignore non-finite values, too.

        if sample.is_infinite() {
            self.infinities += 1;
            return;
        }

        self.count += 1;

        if self.count == 1 {
            self.mean     = sample;
            self.moment_2 = 0.0;
            self.cubes    = 0.0;
            self.moment_4 = 0.0;
            self.min      = sample;
            self.max      = sample;
        } else {
            let distance_mean     = sample - self.mean;
            let new_mean          = self.mean + distance_mean / self.count as f64;
            let distance_new_mean = sample - new_mean;
            let square_estimate   = distance_mean * distance_new_mean;
            let new_moment_2      = self.moment_2 + square_estimate;
            let new_cubes         = self.cubes + sample.powi(3);
            let new_moment_4      = self.moment_4 + square_estimate * square_estimate;

            self.mean             = new_mean;
            self.moment_2         = new_moment_2;
            self.cubes            = new_cubes;
            self.moment_4         = new_moment_4;
            self.min              = min_f64(self.min, sample);
            self.max              = max_f64(self.max, sample);
        }

        self.histogram.borrow_mut().record(sample);
    }

    fn record_event(&mut self) {
        panic!("RunningFloat::record_event: not supported");
    }

    fn record_event_report(&mut self) -> i64 {
        panic!("RunningFloat::record_event_report: not supported");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("RunningFloat::record_time: not supported");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("RunningFloat::record_interval: not supported");
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self)-> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        "float"
    }

    fn count(&self) -> u64 {
        self.count
    }

    fn log_mode(&self) -> isize {
        panic!("RunningFloat:: log_mode not supported");
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
        let n        = self.count as f64;
        let mean     = self.mean;
        let moment_2 = self.moment_2;
        let cubes    = self.cubes;
        let data     = EstimateData { n, mean, moment_2, cubes };

        let moment_3 = estimate_moment_3(data);

        compute_skewness(self.count, self.moment_2, moment_3)
    }

    fn kurtosis(&self) -> f64 {
        compute_kurtosis(self.count, self.moment_2, self.moment_4)
    }

    fn int_extremes(&self) -> bool {
        false
    }

    fn float_extremes(&self) -> bool {
        true
    }

    fn min_i64(&self) -> i64 {
        panic!("RunningFloat::min_i64: not supported");
    }

    fn min_f64(&self) -> f64 {
        self.min
    }

    fn max_i64(&self) -> i64 {
        panic!("RunningFloat::max_i64: not supported");
    }

    fn max_f64(&self) -> f64 {
        self.max
    }

    fn precompute(&mut self) {
    }

    fn clear(&mut self) {
        self.count    = 0;
        self.mean     = 0.0;
        self.moment_2 = 0.0;
        self.cubes    = 0.0;
        self.moment_4 = 0.0;
        self.min      = f64::MAX;
        self.max      = f64::MIN;

        self.histogram.borrow_mut().clear();
    }

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer =
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
        let printer   = printer_mut!(printer);

        printer.print(title);
        printable.print_common_f64(printer);
        printable.print_common_float(printer);
        self.histogram.borrow().print(printer);
        printer.print("");
    }

    fn set_title (&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        None
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        Some(self.histogram.clone())
    }

    // Methods for internal use.

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<RunningFloat>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn export_stats(&self) -> ExportStats {
        let printable       = self.get_printable();
        let log_histogram   = None;
        let float_histogram = Some(self.histogram.clone());

        ExportStats {printable, log_histogram, float_histogram }
    }
}

impl Histogram for RunningFloat {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        self.histogram.borrow().print(printer);
    }

    fn clear_histogram(&mut self) {
        self.histogram.borrow_mut().clear();
    }

    fn to_log_histogram  (&self) -> Option<LogHistogramBox> {
        None
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        Some(self.histogram.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrintOpts;
    use crate::stdout_printer;
    use crate::tests::continuing_box;
    use crate::tests::bytes;
    use crate::tests::check_printer_box;
    use crate::counter::Counter;

    fn compute_sum(histogram: &FloatHistogram) -> i64 {
        let mut sum = 0;

        for sample in histogram.positive.iter() {
            sum += *sample;
        }

        for sample in histogram.negative.iter() {
            sum += *sample;
        }

        sum as i64
    }

    fn simple_float_test() {
        let mut float = RunningFloat::new("Test Statistic", &None);
        let     end   = 1000;

        assert!( float.float_extremes());
        assert!(!float.int_extremes  ());

        for i in 1..=end {
            float.record_f64(i as f64);
        }

        // Compute the expected mean.

        let float_end = end as f64;
        let sum       = (float_end * (float_end + 1.0)) / 2.0;
        let mean      = sum / float_end;

        assert!(float.count()   == end as u64);
        assert!(float.mean()    == mean      );
        assert!(float.min_f64() == 1.0       );
        assert!(float.max_f64() == float_end );

        assert!( float.float_extremes());
        assert!(!float.int_extremes  ());

        float.print();

        float.record_f64(f64::INFINITY);
        float.record_f64(f64::NEG_INFINITY);
        float.record_f64(f64::NAN);

        // NaNs should be counted but then ignored.
        // Same for other non-finite values.

        assert!(float.count()      == end as u64);
        assert!(float.nans()       == 1);
        assert!(float.infinities() == 2);

        // precompute should be a safe no-op.

        float.precompute();

        assert!(float.count()      == end as u64);
        assert!(float.nans()       == 1);
        assert!(float.infinities() == 2);

        let stats = float.export_stats();

        assert!(stats.printable.n            == end as u64);
        assert!(stats.printable.nans         == 1);
        assert!(stats.printable.infinities   == 2);

        let histogram = stats.float_histogram.unwrap();

        let     printer = stdout_printer();
        let     printer = printer_mut!(printer);

        histogram.borrow().print(printer);

        let samples = compute_sum(&histogram.borrow());

        assert!(samples == stats.printable.n as i64);

        float.clear();

        assert!(float.mean()       == 0.0);
        assert!(float.skewness()   == 0.0);
        assert!(float.kurtosis()   == 0.0);
        assert!(float.count()      == 0  );
        assert!(float.nans()       == 1  );
        assert!(float.infinities() == 2  );
    }

    fn test_standard_deviation() {
        let mut float = RunningFloat::new("Test Statistic", &None);

        for _i in 1..100 {
            float.record_f64(1.0);
        }

        assert!(float.standard_deviation() == 0.0);
    }

    #[test]
    #[should_panic]
    fn test_export_log_histogram() {
        let float = RunningFloat::new("Test Statistic", &None);

        let stats = float.export_stats();

        let _     = stats.log_histogram.unwrap();
    }

    #[test]
    #[should_panic]
    fn test_record_i64() {
        let mut float = RunningFloat::new("Test Statistic", &None);

        float.record_i64(1);
    }

    #[test]
    #[should_panic]
    fn test_record_event() {
        let mut float = RunningFloat::new("Test Statistic", &None);

        float.record_event();
    }

    #[test]
    #[should_panic]
    fn test_record_event_report() {
        let mut float = RunningFloat::new("Test Statistic", &None);

        let _ = float.record_event_report();
    }

    #[test]
    #[should_panic]
    fn test_record_time() {
        let mut float = RunningFloat::new("Test Statistic", &None);

        float.record_time(1);
    }

    #[test]
    #[should_panic]
    fn test_log_mode() {
        let float = RunningFloat::new("Test Statistic", &None);

        let _ = float.log_mode();
    }

    #[test]
    #[should_panic]
    fn test_min_i64() {
        let float = RunningFloat::new("Test Statistic", &None);

        let _ = float.min_i64();
    }

    #[test]
    #[should_panic]
    fn test_max_i64() {
        let float = RunningFloat::new("Test Statistic", &None);

        let _ = float.max_i64();
    }

    fn test_equality() {
        let stat_1 = RunningFloat::new("Equality Statistic 1", &None);
        let stat_2 = RunningFloat::new("Equality Statistic 2", &None);
        let stat_3 = Counter::new     ("Equality Statistic 3", &None);

        assert!( stat_1.equals(&stat_1));
        assert!(!stat_1.equals(&stat_2));
        assert!(!stat_1.equals(&stat_3));
    }

    #[test]
    #[should_panic]
    fn test_record_interval() {
        let mut float = RunningFloat::new("Test Statistic", &None);
        let mut timer = continuing_box();

        float.record_interval(&mut timer);
    }

    #[test]
    #[should_panic]
    fn test_to_log_histogram() {
        let float = RunningFloat::new("Test Statistic", &None);

        let _ = float.to_log_histogram().unwrap();
    }

    fn test_title() {
        let mut float      = RunningFloat::new("Test Statistic", &None);
        let     test_title = "set_title Test";

        let start_title = float.title();

        float.set_title(test_title);

        assert!(&float.title() == test_title);
        assert!(test_title     != start_title);
    }

    fn test_histogram() {
        let mut float      = RunningFloat::new("Test Statistic", &None);
        // let     histogram  = float.to_float_histogram().unwrap();
        // let mut histogram  = histogram.borrow_mut();
        let     samples    = 1000;

        for i in 1..=samples {
            float.record_f64(i as f64);
        }

        {
            let histogram = float.to_float_histogram().unwrap();
            let histogram = &*histogram.borrow();

            let sum = compute_sum(histogram);
            assert!(sum == samples);
        }

        float.clear_histogram();

        {
            let histogram = float.to_float_histogram().unwrap();
            let histogram = &*histogram.borrow();

            let sum = compute_sum(histogram);
            assert!(sum == 0);
        }
    }

    fn test_print_output() {
        let expected =
            [
                "Test Statistics",
                "    Count               1,000 ",
                "    NaNs                    0 ",
                "    Infinities              0 ",
                "    Minimum          +1.00000 e+0  byte",
                "    Maximum          +1.00000 e+3  bytes",
                "    Mode Value       +3.84000 e+2  bytes",
                "    Mean             +5.00500 e+2  bytes",
                "    Std Dev          +2.88819 e+2  bytes",
                "    Variance         +8.34166 e+4  ",
                "    Skewness         -4.16317 e-11 ",
                "    Kurtosis         -1.19999 e+0  ",
                "  Float Histogram:  (0 NaN, 0 infinite, 1000 samples)",
                "  -----------------------",
                "    2^  -63:             0             0             0             1",
                "    2^    1:           999             0             0             0",
                ""
            ];

        let     printer    = Some(check_printer_box(&expected, true, false));
        let     title      = None;
        let     units      = bytes();
        let     histo_opts = None;
        let     print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let     name       = "Test Statistics";
        let mut stats      = RunningFloat::new(&name, &print_opts);
        let     samples    = 1000;

        for i in 1..=samples {
            stats.record_f64(i as f64);
        }

        stats.print();
    }

    #[test]
    fn run_tests() {
        simple_float_test();
        test_standard_deviation();
        test_equality();
        test_title();
        test_histogram();
        test_print_output();
    }
}
