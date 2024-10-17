//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::any::Any;
use std::cmp::min;
use std::cmp::max;

use super::Rustics;
use super::Histogram;
use super::TimerBox;
use super::PrinterBox;
use super::PrinterOption;
use super::printable::Printable;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::stdout_printer;
use super::sum::kbk_sum;

use crate::hier::HierExporter;
use crate::LogHistogram;

// Define the implementation of a very simple running integer sample space.

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
}

// RunningExporter structs are used to export statistics from a RunningInteger
// struct so that multiple structures can be summed.

#[derive(Default)]
pub struct RunningExporter {
    addends: Vec<RunningExport>,
}

impl RunningExporter {
    pub fn new() -> RunningExporter {
        let addends = Vec::new();

        RunningExporter { addends }
    }

    pub fn push(&mut self, addend: RunningExport) {
        self.addends.push(addend);
    }

    // Make a member based on the summed exports.

    pub fn make_member(&mut self, name: &str, printer: PrinterBox) -> RunningInteger {
        let title   = name;
        let sum     = sum_running(&self.addends);
        let printer = Some(printer);

        RunningInteger::new_from_exporter(name, title, printer, sum)
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

pub fn sum_log_histogram(sum:  &mut LogHistogram, addend: &LogHistogram) {
    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }
}

// Merge the vector of exported statistics.  Many fields are just
// sums of the parts.

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
    pub fn new(name_in: &str, printer: PrinterOption) -> RunningInteger {
        let name            = String::from(name_in);
        let title           = String::from(name_in);
        let id              = usize::MAX;
        let count           = 0;
        let mean            = 0.0;
        let moment_2        = 0.0;
        let moment_3        = 0.0;
        let moment_4        = 0.0;
        let min             = i64::MAX;
        let max             = i64::MIN;
        let log_histogram   = LogHistogram::new();

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer
        }
    }

    pub fn new_from_exporter(name: &str, title: &str, printer: PrinterOption, import: RunningExport)
            -> RunningInteger {
        let name            = String::from(name);
        let title           = String::from(title);
        let id              = usize::MAX;
        let count           = import.count;
        let mean            = import.mean;
        let moment_2        = import.moment_2;
        let moment_3        = import.moment_3;
        let moment_4        = import.moment_4;
        let min             = import.min;
        let max             = import.max;
        let log_histogram   = import.log_histogram;

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RunningInteger {
            name,       title,      id,
            count,      mean,       moment_2,
            moment_3,   moment_4,   log_histogram,
            min,        max,        printer
        }
    }

    // Export all the statistics from a given structure to
    // be used to create a sum of many structures.

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

    fn histo_log_mode(&self) -> i64 {
        self.log_histogram.log_mode() as i64
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

        let n        = self.count;
        let min      = self.min;
        let max      = self.max;
        let log_mode = self.log_histogram.log_mode() as i64;
        let mean     = self.mean;
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        println!("print_opts:  getting printer lock");
        let printer  = &mut *printer_box.lock().unwrap();
        println!("print_opts:  got printer lock");

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

    fn print_histogram(&self) {
        let printer = &mut *self.printer.lock().unwrap();
        self.log_histogram.print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestPrinter;
    use std::sync::Mutex;
    use std::sync::Arc;

    pub fn test_simple_running_integer() {
        let mut stats = RunningInteger::new(&"Test Statistics", None);

        for sample in -256..512 {
            stats.record_i64(sample);
        }

        let printer = Arc::new(Mutex::new(TestPrinter::new("test header ======")));
        stats.print_opts(Some(printer), None);
    }
    
    #[test]
    fn run_tests() {
        test_simple_running_integer();
    }
}
