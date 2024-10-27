//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
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
//!    use rustics::running_integer::RunningInteger;
//!
//!    // Create an instance to record packet sizes.  The default for
//!    // printing output is stdout, which we'll assume is fine for this
//!    // example, so None works for the printer.
//!
//!    let mut packet_sizes = RunningInteger::new("Packet Sizes", None);
//!
//!    // Record some hypothetical packet sizes.
//!
//!    let sample_count = 1000;
//!
//!    for i in 1..sample_count + 1 {
//!       packet_sizes.record_i64(i as i64);
//!       assert!(packet_sizes.count() == i as u64);
//!    }
//!
//!    // Print our statistics.
//!
//!    packet_sizes.print();
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
//!    for i in 1..next_sample_count + 1 {
//!       packet_sizes.record_i64(i + sample_count as i64);
//!       assert!(packet_sizes.count() == (sample_count + i) as u64);
//!    }
//!```

use std::any::Any;
use std::cmp::min;
use std::cmp::max;

use super::Rustics;
use super::Histogram;
use super::TimerBox;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::PrintOpts;
use super::Units;
use super::printable::Printable;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::sum::kbk_sum;

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
    moment_3:   f64,
    moment_4:   f64,

    min:        i64,
    max:        i64,

    pub log_histogram:    LogHistogram,

    printer:    PrinterBox,
    units:      Units,
}

// RunningExporter instances are used to export statistics from a
// RunningInteger instance so that multiple RunningInteger instances
// can be summed.  This is used by IntegerHier to allow the Hier
// code to use RunningInteger instance.  The RunningTime code uses
// a RunningInteger instance underneath a wrapper, so TimeHier uses this
// code, as well.

/// RunningExport mostly is for internal use.  It is available for
/// general use, but most commonly, it will be used by a Hier instance
/// to make summations of statistics instances.

#[derive(Clone, Default)]
pub struct RunningExporter {
    addends: Vec<RunningExport>,
}

/// RunningExporter is intend mostly for internal use by Hier instances.
/// It is used to sum a list of RunningInteger statistics instances.

impl RunningExporter {
    /// Creates a new RunningExporter instance

    pub fn new() -> RunningExporter {
        let addends = Vec::new();

        RunningExporter { addends }
    }

    /// Pushes a statistics instance onto the list of instances to
    /// be summed.

    pub fn push(&mut self, addend: RunningExport) {
        self.addends.push(addend);
    }

    /// Makes a member statistics instance based on the summed exports.

    pub fn make_member(&mut self, name: &str, print_opts: &PrintOption) -> RunningInteger {
        let title   = name;
        let sum     = sum_running(&self.addends);

        RunningInteger::new_from_exporter(name, title, print_opts, sum)
    }
}

// The Hier code uses this trait to do summation of statistics.
//
// We just need downcasting capabilities since all the work
// is implementation-specific.

impl HierExporter for RunningExporter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// RunningExport is used by various modules to create sums of
/// statistics instances of type RunningInteger.

#[derive(Clone)]
pub struct RunningExport {
    pub count:      u64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub moment_3:   f64,
    pub moment_4:   f64,

    pub min:        i64,
    pub max:        i64,

    pub log_histogram:    LogHistogram,
}

/// sum_log_histogram() is used internally to create sums of
/// RunningInteger instances.

pub fn sum_log_histogram(sum:  &mut LogHistogram, addend: &LogHistogram) {
    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }
}

/// The sum_running() function merges a vector of exported statistics.

pub fn sum_running(exports: &Vec::<RunningExport>) -> RunningExport {
    let mut count          = 0;
    let mut min            = i64::MAX;
    let mut max            = i64::MIN;
    let mut log_histogram  = LogHistogram::new();

    let mut mean_vec       = Vec::with_capacity(exports.len());
    let mut moment_2_vec   = Vec::with_capacity(exports.len());
    let mut moment_3_vec   = Vec::with_capacity(exports.len());
    let mut moment_4_vec   = Vec::with_capacity(exports.len());

    for export in exports {
        count    += export.count;
        min       = std::cmp::min(min, export.min);
        max       = std::cmp::max(max, export.max);

        sum_log_histogram(&mut log_histogram, &export.log_histogram);

        mean_vec.push(export.mean * export.count as f64);
        moment_2_vec.push(export.moment_2);
        moment_3_vec.push(export.moment_3);
        moment_4_vec.push(export.moment_4);
    }

    let mean     = kbk_sum(&mut mean_vec[..]) / count as f64;
    let moment_2 = kbk_sum(&mut moment_2_vec[..]);
    let moment_3 = kbk_sum(&mut moment_3_vec[..]);
    let moment_4 = kbk_sum(&mut moment_4_vec[..]);

    RunningExport { count, mean, moment_2, moment_3, moment_4, min, max, log_histogram }
}

impl RunningInteger {
    /// Creates a new RunningInteger instance with the given name and
    /// an optional print function.

    pub fn new(name: &str, printer: PrinterOption) -> RunningInteger {
        let units      = None;
        let title      = None;
        let print_opts = PrintOpts { printer, title, units };
        let print_opts = Some(print_opts);

        RunningInteger::new_opts(name, &print_opts)
    }

    pub fn new_opts(name: &str, print_opts: &PrintOption) -> RunningInteger {
        let (printer, title, units) = parse_print_opts(print_opts, name);

        RunningInteger::new_parsed(name, printer, title, units)
    }

    fn new_parsed(name: &str, printer: PrinterBox, title: String, units: Units) -> RunningInteger {
        let name            = name.to_string();
        let id              = usize::MAX;
        let count           = 0;
        let mean            = 0.0;
        let moment_2        = 0.0;
        let moment_3        = 0.0;
        let moment_4        = 0.0;
        let min             = i64::MAX;
        let max             = i64::MIN;
        let log_histogram   = LogHistogram::new();

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer,
            units
        }
    }

    /// Creates a RunningInteger instance from data from a list of
    /// instances.

    pub fn new_from_exporter(name: &str, title: &str, print_opts: &PrintOption, import: RunningExport)
            -> RunningInteger {
        let (printer, _title, units) = parse_print_opts(print_opts, name);

        let name            = String::from(name);
        let title           = title.to_string();
        let id              = usize::MAX;
        let count           = import.count;
        let mean            = import.mean;
        let moment_2        = import.moment_2;
        let moment_3        = import.moment_3;
        let moment_4        = import.moment_4;
        let min             = import.min;
        let max             = import.max;
        let log_histogram   = import.log_histogram;

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer,
            units
        }
    }

    /// Exports all the statistics kept for a given instance to
    /// be used to create a sum of many instances.

    pub fn export(&self) -> RunningExport {
        let count           = self.count;
        let mean            = self.mean;
        let moment_2        = self.moment_2;
        let moment_3        = self.moment_3;
        let moment_4        = self.moment_4;
        let log_histogram   = self.log_histogram.clone();
        let min             = self.min;
        let max             = self.max;

        RunningExport {
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max
        }
    }

    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }
}

// The formula for computing the second moment for the variance (moment_2)
// is from D. E. Knuth, The Art of Computer Programming.

impl Rustics for RunningInteger {
    fn record_i64(&mut self, sample: i64) {
        self.count += 1;

        self.log_histogram.record(sample);

        let sample_f64 = sample as f64;

        if self.count == 1 {
            self.mean     = sample_f64;
            self.moment_2 = 0.0;
            self.moment_3 = 0.0;
            self.moment_4 = 0.0;
            self.min      = sample;
            self.max      = sample;
        } else {
            let distance_mean     = sample_f64 - self.mean;
            let new_mean          = self.mean + distance_mean / self.count as f64;
            let distance_new_mean = sample_f64 - new_mean;
            let square_estimate   = distance_mean * distance_new_mean;
            let cube_estimate     = square_estimate * square_estimate.sqrt();
            let new_moment_2      = self.moment_2 + square_estimate;
            let new_moment_3      = self.moment_3 + cube_estimate;
            let new_moment_4      = self.moment_4 + square_estimate * square_estimate;

            self.mean             = new_mean;
            self.moment_2         = new_moment_2;
            self.moment_3         = new_moment_3;
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
        self.log_histogram.log_mode()
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

    fn precompute(&mut self) {
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn min_i64(&self) -> i64 {
        self.min
    }

    fn max_i64(&self) -> i64 {
        self.max
    }

    fn min_f64(&self) -> f64 {
        self.min as f64
    }

    fn max_f64(&self) -> f64 {
        self.max as f64
    }

    fn clear(&mut self) {
        self.count    = 0;
        self.mean     = 0.0;
        self.moment_2 = 0.0;
        self.moment_3 = 0.0;
        self.moment_4 = 0.0;
        self.min      = i64::MIN;
        self.max      = i64::MAX;

        self.log_histogram.clear();
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

    fn histogram(&self) -> LogHistogram {
        self.log_histogram.clone()
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

        let n         = self.count;
        let min       = self.min;
        let max       = self.max;
        let log_mode  = self.log_histogram.log_mode() as i64;
        let mean      = self.mean;
        let variance  = self.variance();
        let skewness  = self.skewness();
        let kurtosis  = self.kurtosis();
        let units     = self.units.clone();

        let printable =
            Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis, units };

        let printer   = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_integer(printer);
        printable.print_common_float(printer);
        self.log_histogram.print(printer);
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

impl Histogram for RunningInteger {
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
    use crate::tests::TestPrinter;
    use std::sync::Mutex;
    use std::sync::Arc;
    use crate::log_histogram::pseudo_log_index;
    use super::PrintOpts;

    pub fn test_simple_running_integer() {
        let     printer    = None;
        let     title      = None;

        let     singular   = "byte" .to_string();
        let     plural     = "bytes".to_string();
        let     units      = Some(Units { singular, plural });
        let     print_opts = Some(PrintOpts { printer, title, units });

        let     name       = "Test Statistics";
        let     title      = "Test Title";
        let     id         = 42;
        let mut stats      = RunningInteger::new_opts(&name, &print_opts);
        let mut events     =    0;
        let     min        = -256;
        let     max        =  511;


        assert!(stats.name()  == name);
        assert!(stats.title() == name);
        assert!(stats.class() == "integer");
        assert!(stats.id()    == usize::MAX);

        assert!(stats.equals(&stats));
        assert!(stats.int_extremes());

        stats.set_title(title);
        stats.set_id   (id   );

        assert!(stats.title() == title);
        assert!(stats.id()    == id   );

        for sample in min..max + 1 {
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
        let printer = Arc::new(Mutex::new(printer));

        stats.print_opts(Some(printer), None);

        // Test that the log mode makes sense.

        let common_value = 128;

        for _i in 0..10000 {
            stats.record_i64(common_value);
            events += 1;
        }

        println!("log mode index {}", pseudo_log_index(common_value));
        println!("log mode {}", stats.log_mode());
        println!("log mode {}", stats.log_mode());
        assert!(stats.log_mode() == 7);
    }

    #[test]
    fn run_tests() {
        test_simple_running_integer();
    }
}
