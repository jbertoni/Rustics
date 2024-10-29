//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::Rustics;
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
use super::min_f64;
use super::max_f64;
use super::min_exponent;
use super::max_exponent;

use super::float_histogram::Suppress;
use super::float_histogram::SuppressOption;

pub struct RunningFloat {
    name:      String,
    id:        usize,
    count:     u64,
    mean:      f64,
    moment_2:  f64,
    moment_3:  f64,
    moment_4:  f64,
    min:       f64,
    max:       f64,
    title:     String,
    units:     Units,
    histogram: FloatHistogramBox,
    printer:   PrinterBox,
}

impl RunningFloat {
    pub fn new(name: &str, print_opts: PrintOption, suppress: SuppressOption) -> RunningFloat {
        let (printer, title, units) = parse_print_opts(&print_opts, name);

        let suppress =
            if let Some(suppress) = suppress {
                suppress
            } else {
                let min = min_exponent();
                let max = max_exponent();

                Suppress { min, max }
            };

        let name      = name.to_string();
        let id        = usize::MAX;
        let count     = 0;
        let min       = f64::MAX;
        let max       = f64::MIN;
        let mean      = 0.0;
        let moment_2  = 0.0;
        let moment_3  = 0.0;
        let moment_4  = 0.0;
        let histogram = FloatHistogram::new(suppress);
        let histogram = Rc::from(RefCell::new(histogram));

        RunningFloat {
            name, id, count, mean, moment_2, moment_3, moment_4, max, min, title, units, printer,
            histogram
        }
    }

    fn get_printable(&self) -> Printable {
        let n         = self.count;
        let min_i64   = i64::MIN;
        let max_i64   = i64::MAX;
        let min_f64   = self.min;
        let max_f64   = self.max;
        let log_mode  = self.histogram.borrow().log_mode() as i64;
        let mean      = self.mean;
        let variance  = self.variance();
        let skewness  = self.skewness();
        let kurtosis  = self.kurtosis();
        let units     = self.units.clone();

        Printable {
            n,    min_i64,   max_i64,   min_f64,   max_f64,  log_mode,
            mean, variance,  skewness,  kurtosis,  units
        }
    }
}

impl Rustics for RunningFloat {
    fn record_i64(&mut self, _sample: i64) {
        panic!("RunningFloat::record_i64: not supported");
    }

    fn record_f64(&mut self, sample: f64) {
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
        self.histogram.borrow().log_mode()
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
        let printable = self.get_printable();
        let log_histogram = None;
        let float_histogram = Some(self.histogram.clone());

        ExportStats {printable, log_histogram, float_histogram }
    }
}
