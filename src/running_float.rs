//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!  
//! ## Type
//! * RunningFloat
//!   * RunningFloat provides statistical summaries of samples of type f64.
//!
//!   * This includes a very coarse log histogram very similar to the one
//!     that supports i64 data.
//!
//! ## Example
//!```
//!     use rustics::Rustics;
//!     use rustics::ExportStats;
//!     use rustics::printable::Printable;
//!     use rustics::running_float::RunningFloat;
//!
//!     let mut float = RunningFloat::new("Test Statistic", &None);
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
use super::compute_skewness;
use super::compute_kurtosis;
use super::FloatHistogram;
use super::FloatHistogramBox;
use super::sum::kbk_sum_sort;
use super::min_f64;
use super::max_f64;

#[derive(Clone)]
pub struct FloatExport {
    pub count:      u64,
    pub nans:       u64,
    pub infinities: u64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,

    pub min:        f64,
    pub max:        f64,

    pub histogram:  FloatHistogramBox,
}

// FloatExporter instances are used to export statistics from a
// RunningFloat instance so that multiple RunningFloat instances can
// be summed.  This is used by FloatHier to allow the Hier code to use
// RunningFloat instances.

/// FloatExport mostly is for internal use.  It is available for
/// general use, but most commonly, it will be used by a Hier instance
/// to make summations of statistics instances.

#[derive(Clone, Default)]
pub struct FloatExporter {
    addends: Vec<FloatExport>,
}

/// FloatExporter is intend mostly for internal use by Hier instances.
/// It is used to sum a list of RunningInteger statistics instances.

impl FloatExporter {
    /// Creates a new FloatExporter instance

    pub fn new() -> FloatExporter {
        let addends = Vec::new();

        FloatExporter { addends }
    }

    /// Pushes a statistics instance onto the list of instances to
    /// be summed.

    pub fn push(&mut self, addend: FloatExport) {
        self.addends.push(addend);
    }

    /// Makes a member statistics instance based on the summed exports.

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

/// sum_float_histogram() is used internally to create sums of
/// RunningFloat instances.

pub fn sum_float_histogram(sum:  &mut FloatHistogram, addend: &FloatHistogram) {
    assert!(sum.negative.len() == addend.negative.len());
    assert!(sum.positive.len() == addend.positive.len());

    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }

    sum.nans       += addend.nans;
    sum.infinities += addend.infinities;
    sum.samples    += addend.samples;
}

/// The sum_running() function merges a vector of exported statistics.

pub fn sum_running(exports: &Vec::<FloatExport>) -> FloatExport {
    let mut count          = 0;
    let mut nans           = 0;
    let mut infinities     = 0;
    let mut min            = f64::MAX;
    let mut max            = f64::MIN;
    let     print_opts     = &exports[0].histogram.borrow().print_opts;
    let mut histogram      = FloatHistogram::new(print_opts);

    let mut mean_vec       = Vec::with_capacity(exports.len());
    let mut moment_2_vec   = Vec::with_capacity(exports.len());
    let mut moment_3_vec   = Vec::with_capacity(exports.len());
    let mut moment_4_vec   = Vec::with_capacity(exports.len());

    for export in exports {
        count      += export.count;
        nans       += export.nans;
        infinities += export.infinities;

        if export.min < min {
            min = export.min;
        }

        if export.max > max {
            max = export.max;
        }

        sum_float_histogram(&mut histogram, &export.histogram.borrow());

        mean_vec.push(export.mean * export.count as f64);
        moment_2_vec.push(export.moment_2);
        moment_3_vec.push(export.moment_3);
        moment_4_vec.push(export.moment_4);
    }

    let mean       = kbk_sum_sort(&mut mean_vec[..]) / count as f64;
    let moment_2   = kbk_sum_sort(&mut moment_2_vec[..]);
    let moment_3   = kbk_sum_sort(&mut moment_3_vec[..]);
    let moment_4   = kbk_sum_sort(&mut moment_4_vec[..]);
    let histogram  = Rc::from(RefCell::new(histogram));

    FloatExport {
        count,  nans,   infinities, mean, moment_2, moment_3, moment_4,
        min,    max,    histogram
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
    moment_3:   f64,
    moment_4:   f64,
    min:        f64,
    max:        f64,
    title:      String,
    units:      Units,
    histogram:  FloatHistogramBox,
    printer:    PrinterBox,
}

impl RunningFloat {
    /// Constructs a new statistics type.  print_opts and histo_opts affect how
    /// the output of print functions looks.  "None" will accept the defaults,
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
        let moment_3    = 0.0;
        let moment_4    = 0.0;
        let histogram   = FloatHistogram::new(print_opts);
        let histogram   = Rc::from(RefCell::new(histogram));

        RunningFloat {
            name,      id,        count,    nans,   infinities,  mean,   moment_2,
            moment_3,  moment_4,  max,      min,    title,       units,  printer,
            histogram
        }
    }

    pub fn new_from_exporter(name: &str, title: &str, print_opts: &PrintOption, import: FloatExport)
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
        let moment_3   = import.moment_3;
        let moment_4   = import.moment_4;
        let min        = import.min;
        let max        = import.max;
        let histogram  = import.histogram;

        RunningFloat {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   histogram,
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

    /// Exports all the statistics kept for a given instance to
    /// be used to create a sum of many instances.

    pub fn export_data(&self) -> FloatExport {
        let count           = self.count;
        let nans            = self.nans;
        let infinities      = self.infinities;
        let mean            = self.mean;
        let moment_2        = self.moment_2;
        let moment_3        = self.moment_3;
        let moment_4        = self.moment_4;
        let histogram       = self.histogram.clone(); 
        let min             = self.min;
        let max             = self.max;

        FloatExport { 
            count,      nans,       infinities,
            mean,       moment_2,   moment_3,
            moment_4,   histogram,  min,
            max
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
            self.moment_3 = 0.0;
            self.moment_4 = 0.0;
            self.min      = sample;
            self.max      = sample;
        } else {
            let distance_mean     = sample - self.mean;
            let new_mean          = self.mean + distance_mean / self.count as f64;
            let distance_new_mean = sample - new_mean;
            let square_estimate   = distance_mean * distance_new_mean;
            let cube_estimate     = square_estimate * square_estimate.sqrt();
            let new_moment_2      = self.moment_2 + square_estimate;
            let new_moment_3      = self.moment_3 + cube_estimate;
            let new_moment_4      = self.moment_4 + square_estimate * square_estimate;

            self.mean             = new_mean;
            self.moment_2         = new_moment_2;
            self.moment_3         = new_moment_3;
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
        compute_skewness(self.count, self.moment_2, self.moment_3)
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
        self.moment_3 = 0.0;
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
        let printer   = &mut *printer.lock().unwrap();

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
    use crate::stdout_printer;
    use crate::tests::continuing_box;
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
        let mut printer = printer.lock().unwrap();
        let     printer = &mut *printer;

        histogram.borrow().print(printer);

        let samples = compute_sum(&histogram.borrow());

        assert!(samples == stats.printable.n as i64);
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

    #[test]
    fn run_tests() {
        simple_float_test();
        test_standard_deviation();
        test_equality();
        test_title();
        test_histogram();
    }
}
