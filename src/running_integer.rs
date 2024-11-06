//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//! ## Type
//!
//! * RunningInteger
//!     * RunningInteger maintains running statistics on a set of samples
//!       recorded into it.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use rustics::Rustics;
//!    use rustics::Histogram;
//!    use rustics::printer;
//!    use rustics::stdout_printer;
//!    use rustics::running_integer::RunningInteger;
//!
//!    // Create an instance to record packet sizes.  The default for
//!    // printing output is stdout, which we'll assume is fine for this
//!    // example, so None works for the printer.
//!
//!    let mut packet_sizes = RunningInteger::new("Packet Sizes", &None);
//!
//!    // Record some hypothetical packet sizes.
//!
//!    let sample_count = 1000;
//!
//!    for i in 1..=sample_count {
//!       packet_sizes.record_i64(i as i64);
//!       assert!(packet_sizes.count() == i as u64);
//!    }
//!
//!    // Print our statistics.
//!
//!    packet_sizes.print();
//!
//!    // Print just the histogram.  This example shows how PrinterBox
//!    // the printer code work.
//!
//!    let printer = stdout_printer();  // create a shareable printer
//!    let printer = printer!(printer); // get the printer out of the cell
//!
//!    packet_sizes.print_histogram(printer);
//!
//!    // We should have seen "sample_count" events.
//!
//!    assert!(packet_sizes.count() == sample_count as u64);
//!
//!    // Compute the expected mean.  We need the sum of
//!    //     1 + 2 + ... + n
//!    // which is
//!    //     n * (n + 1) / 2.
//!
//!    let float_count = sample_count as f64;
//!    let float_sum   = float_count * (float_count + 1.0) / 2.0;
//!    let mean        = float_sum / float_count;
//!
//!    assert!(packet_sizes.mean() == mean);
//!
//!    // Let's record more samples, and verify the sample count as we go.
//!
//!    let next_sample_count = 100;
//!
//!    for i in 1..=next_sample_count {
//!       packet_sizes.record_i64(i + sample_count as i64);
//!       assert!(packet_sizes.count() == (sample_count + i) as u64);
//!    }
//!```

use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::min;
use std::cmp::max;

use super::Rustics;
use super::Histogram;
use super::TimerBox;
use super::Printer;
use super::ExportStats;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use super::Units;
use super::printer;
use super::printable::Printable;
use super::EstimateData;
use super::estimate_moment_3;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::merge::Export;
use super::merge::sum_running;

use crate::hier::HierExporter;
use crate::LogHistogram;

use super::parse_print_opts;

/// RunningInteger provides very simple statistics on a
/// stream of integer data samples.
///
/// See the module comments for a sample program.

#[derive(Clone)]
pub struct RunningInteger {
    name:       String,
    title:      String,
    id:         usize,

    count:      u64,
    mean:       f64,
    moment_2:   f64,
    cubes:      f64,
    moment_4:   f64,

    min:        i64,
    max:        i64,

    log_histogram:  LogHistogramBox,

    printer:    PrinterBox,
    units:      Units,
}

// IntegerExporter instances are used to export statistics from a
// RunningInteger instance so that multiple RunningInteger instances
// can be summed.  This is used by IntegerHier to allow the Hier
// code to use RunningInteger instance.  The RunningTime code uses
// a RunningInteger instance underneath a wrapper, so TimeHier uses this
// code, as well.

/// IntegerExport mostly is for internal use.  It is available for
/// general use, but most commonly, it will be used by a Hier instance
/// to make summations of statistics instances.

#[derive(Clone, Default)]
pub struct IntegerExporter {
    addends: Vec<Export>,
}

/// IntegerExporter is intend mostly for internal use by Hier instances.
/// It is used to sum a list of RunningInteger statistics instances.

impl IntegerExporter {
    /// Creates a new IntegerExporter instance

    pub fn new() -> IntegerExporter {
        let addends = Vec::new();

        IntegerExporter { addends }
    }

    /// Pushes a statistics instance onto the list of instances to
    /// be summed.

    pub fn push(&mut self, addend: Export) {
        self.addends.push(addend);
    }

    /// Makes a member statistics instance based on the summed exports.

    pub fn make_member(&mut self, name: &str, print_opts: &PrintOption) -> RunningInteger {
        let title = name;
        let sum   = sum_running(&self.addends);

        RunningInteger::new_from_exporter(name, title, print_opts, sum)
    }

    // For testing

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.addends.len()
    }
}

// The Hier code uses this trait to do summation of statistics.
//
// We just need downcasting capabilities since all the work
// is implementation-specific.

impl HierExporter for IntegerExporter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl RunningInteger {
    /// Creates a new RunningInteger instance with the given name and
    /// an optional set of print options.

    pub fn new(name: &str, print_opts: &PrintOption) -> RunningInteger {
        let (printer, title, units, _histo_opts) = parse_print_opts(print_opts, name);

        let name            = name.to_string();
        let id              = usize::MAX;
        let count           = 0;
        let mean            = 0.0;
        let moment_2        = 0.0;
        let cubes           = 0.0;
        let moment_4        = 0.0;
        let min             = i64::MAX;
        let max             = i64::MIN;
        let log_histogram   = LogHistogram::new();
        let log_histogram   = Rc::from(RefCell::new(log_histogram));

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            cubes,      moment_4,   log_histogram,
            min,        max,        printer,
            units
        }
    }

    /// Creates a RunningInteger instance from data from a list of
    /// instances.

    pub fn new_from_exporter(name: &str, title: &str, print_opts: &PrintOption, import: Export)
            -> RunningInteger {
        let (printer, _title, units, _histo_opts) = parse_print_opts(print_opts, name);

        let name            = String::from(name);
        let title           = title.to_string();
        let id              = usize::MAX;
        let count           = import.count;
        let mean            = import.mean;
        let moment_2        = import.moment_2;
        let cubes           = import.cubes;
        let moment_4        = import.moment_4;
        let min             = import.min_i64;
        let max             = import.max_i64;
        let log_histogram   = import.log_histogram.unwrap();

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            cubes,      moment_4,   log_histogram,
            min,        max,        printer,
            units
        }
    }

    /// Exports all the statistics kept for a given instance to
    /// be used to create a sum of many instances.

    pub fn export_data(&self) -> Export {
        let count           = self.count;
        let nans            = 0;
        let infinities      = 0;
        let mean            = self.mean;
        let moment_2        = self.moment_2;
        let cubes           = self.cubes;
        let moment_4        = self.moment_4;
        let log_histogram   = Some(self.log_histogram.clone());
        let float_histogram = None;
        let min_i64         = self.min;
        let max_i64         = self.max;
        let min_f64         = 0.0;
        let max_f64         = 0.0;

        Export {
            count,      nans,        infinities,
            mean,       moment_2,    cubes,
            moment_4,   min_i64,     max_i64,
            min_f64,    max_f64,     log_histogram,
            float_histogram
        }
    }

    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    pub fn get_printable(&self) -> Printable {
        let n           = self.count;
        let nans        = 0;
        let infinities  = 0;
        let min_i64     = self.min;
        let max_i64     = self.max;
        let min_f64     = f64::MIN;
        let max_f64     = f64::MAX;
        let log_mode    = self.log_histogram.borrow().log_mode() as i64;
        let mode_value  = 0.0;
        let mean        = self.mean;
        let variance    = self.variance();
        let skewness    = self.skewness();
        let kurtosis    = self.kurtosis();
        let units       = self.units.clone();

        Printable {
            n,         nans,  infinities,  min_i64,   max_i64,   min_f64,     max_f64,
            log_mode,  mean,  variance,    skewness,  kurtosis,  mode_value,  units
        }
    }
}

// The formula for computing the second moment for the variance (moment_2)
// is from D. E. Knuth, The Art of Computer Programming.

impl Rustics for RunningInteger {
    fn record_i64(&mut self, sample: i64) {
        self.count += 1;

        self.log_histogram.borrow_mut().record(sample);

        let sample_f64 = sample as f64;

        if self.count == 1 {
            self.mean     = sample_f64;
            self.moment_2 = 0.0;
            self.cubes    = 0.0;
            self.moment_4 = 0.0;
            self.min      = sample;
            self.max      = sample;
        } else {
            let distance_mean     = sample_f64 - self.mean;
            let new_mean          = self.mean + (distance_mean / self.count as f64);
            let distance_new_mean = sample_f64 - new_mean;
            let square_estimate   = (distance_mean * distance_new_mean).abs();
            let new_moment_2      = self.moment_2 + square_estimate;
            let new_cubes         = self.cubes + sample_f64.powi(3);
            let new_moment_4      = self.moment_4 + square_estimate * square_estimate;

            self.mean             = new_mean;
            self.moment_2         = new_moment_2;
            self.cubes            = new_cubes;
            self.moment_4         = new_moment_4;
            self.min              = min(self.min, sample);
            self.max              = max(self.max, sample);
        }
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::RunningInteger:  f64 samples are not permitted.");
    }

    fn record_event(&mut self) {
        panic!("Rustics::RunningInteger:  event samples are not permitted.");
    }

    fn record_event_report(&mut self) -> i64 {
        panic!("Rustics::RunningInteger:  event samples are not permitted.");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Rustics::RunningInteger:  time samples are not permitted.");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Rustics::RunningInteger:  time intervals are not permitted.");
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
        self.count
    }

    fn log_mode(&self) -> isize {
        self.log_histogram.borrow().log_mode()
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

    fn precompute(&mut self) {
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn float_extremes(&self) -> bool {
        false
    }

    fn min_i64(&self) -> i64 {
        self.min
    }

    fn max_i64(&self) -> i64 {
        self.max
    }

    fn min_f64(&self) -> f64 {
        panic!("RunningInteger:: min_f64 is not supported.");
    }

    fn max_f64(&self) -> f64 {
        panic!("RunningInteger:: max_f64 is not supported.");
    }

    fn clear(&mut self) {
        self.count    = 0;
        self.mean     = 0.0;
        self.moment_2 = 0.0;
        self.cubes    = 0.0;
        self.moment_4 = 0.0;
        self.min      = i64::MAX;
        self.max      = i64::MIN;

        self.log_histogram.borrow_mut().clear();
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

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        Some(self.log_histogram.clone())
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }

    fn print(&self) {
        self.print_opts(None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let printer_box =
            if let Some(printer) = printer {
                printer.clone()
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
        let printer   = printer!(printer_box);

        printer.print(title);
        printable.print_common_i64(printer);
        printable.print_common_float(printer);
        self.log_histogram.borrow().print(printer);
        printer.print("");
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

impl Histogram for RunningInteger {
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
mod tests {
    use super::*;
    use crate::PrintOpts;
    use crate::printer_box;
    use crate::counter::Counter;
    use crate::hier::HierMember;
    use crate::tests::continuing_box;
    use crate::tests::TestPrinter;
    use crate::tests::bytes;
    use crate::tests::check_printer_box;

    pub fn test_simple_running_integer() {
        let     printer    = None;
        let     title      = None;
        let     units      = bytes();
        let     histo_opts = None;
        let     print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let     name       = "Test Statistics";
        let     id         = 42;
        let mut stats      = RunningInteger::new(&name, &print_opts);
        let     title      = "Test Title";
        let mut events     =    0;
        let     min        = -256;
        let     max        =  511;

        assert!(stats.name()  == name);
        assert!(stats.title() == name);
        assert!(stats.class() == "integer");
        assert!(stats.id()    == usize::MAX);

        assert!( stats.int_extremes  ());
        assert!(!stats.float_extremes());

        assert!(stats.equals(&stats));
        assert!(stats.int_extremes());

        stats.set_title(title);
        stats.set_id   (id   );

        assert!(stats.title() == title);
        assert!(stats.id()    == id   );

        for sample in min..=max {
            stats.record_i64(sample);
            events += 1;
        }

        let mean = (min + max) as f64 / 2.0;

        assert!(stats.min_i64() == min      );
        assert!(stats.max_i64() == max      );
        assert!(stats.mean()    == mean     );
        assert!(stats.count()   == events   );
        assert!(stats.class()   == "integer");

        let printer = TestPrinter::new("test header ======");
        let printer = printer_box!(printer);

        stats.print_opts(Some(printer), None);

        // Test that the log mode makes sense.

        let common_value = 128;

        for _i in 0..10000 {
            stats.record_i64(common_value);
            events += 1;
        }

        let expected = 7;

        println!("test_simple_running_integer:  log mode {}, expected {}",
            stats.log_mode(), expected);

        assert!(stats.log_mode() == expected);

        stats.clear();
        assert!(stats.min_i64() == i64::MAX);
        assert!(stats.max_i64() == i64::MIN);

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

    #[test]
    #[should_panic]
    fn test_record_f64() {
        let mut stats = RunningInteger::new("Panic Test", &None);

        stats.record_f64(1.0);
    }

    #[test]
    #[should_panic]
    fn test_record_event() {
        let mut stats = RunningInteger::new("Panic Test", &None);

        stats.record_event();
    }

    #[test]
    #[should_panic]
    fn test_record_event_report() {
        let mut stats = RunningInteger::new("Panic Test", &None);

        let _ = stats.record_event_report();
    }

    #[test]
    #[should_panic]
    fn test_record_time() {
        let mut stats = RunningInteger::new("Panic Test", &None);

        stats.record_time(1);
    }

    #[test]
    #[should_panic]
    fn test_record_interval() {
        let mut timer = continuing_box();
        let mut stats = RunningInteger::new("Panic Test", &None);

        stats.record_interval(&mut timer);
    }

    #[test]
    #[should_panic]
    fn test_to_float_histogram() {
        let stats = RunningInteger::new("Panic Test", &None);

        let _ = stats.to_float_histogram().unwrap();
    }

    fn test_equality() {
        let stats_1 = RunningInteger::new("Equality Test 1", &None);
        let stats_2 = RunningInteger::new("Equality Test 2", &None);
        let stats_3 = Counter::       new("Equality Test 3", &None);

        assert!( stats_1.equals(&stats_1));
        assert!(!stats_1.equals(&stats_2));
        assert!(!stats_1.equals(&stats_3));

        let mut stats = RunningInteger::new("Equality Test 1", &None);

        let any       = stats.as_any();
        let any_stats = any.downcast_ref::<RunningInteger>().unwrap();

        assert!(stats.equals(any_stats));

        // Now set_id() and id() to check equality.

        let expected = 12034; // Something unliklely.

        stats.set_id(expected);

        let any       = stats.as_any_mut();
        let any_stats = any.downcast_ref::<RunningInteger>().unwrap();

        assert!(any_stats.id() == expected);
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
                "    Skewness         -4.16317 e-11 ",
                "    Kurtosis         -1.19999 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:               128               256               488                 0",
                ""
            ];

        let     printer    = Some(check_printer_box(&expected, true, false));
        let     title      = None;
        let     units      = bytes();
        let     histo_opts = None;
        let     print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let     name       = "Test Statistics";
        let mut stats      = RunningInteger::new(&name, &print_opts);
        let     samples    = 1000;

        for i in 1..=samples {
            stats.record_i64(i as i64);
        }

        stats.print();
    }

    #[test]
    fn run_tests() {
        test_simple_running_integer();
        test_equality();
        test_print_output();
    }
}
