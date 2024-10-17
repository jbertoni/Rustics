//      
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//      

use std::any::Any;

use super::Rustics;
use super::PrinterBox;
use super::PrinterOption;
use super::TimerBox;
use super::Histogram;
use super::timer_box_hz;
use super::integer_window::IntegerWindow;
use crate::printable::Printable;
use super::compute_variance;
use super::compute_skewness;
use super::compute_kurtosis;
use super::stdout_printer;

pub struct TimeWindow {
    printer:            PrinterBox,

    integer_window:     Box<IntegerWindow>,
    timer:              TimerBox,
    hz:                 i64,
}

impl TimeWindow {
    pub fn new(name_in: &str, window_size: usize, timer:  TimerBox) -> TimeWindow {
        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::TimeWindow:  The timer hz value is too large.");
        }

        let hz             = hz as i64;
        let integer_window = IntegerWindow::new(name_in, window_size);
        let integer_window = Box::new(integer_window);
        let printer        = stdout_printer();

        TimeWindow { printer, integer_window, timer, hz }
    }

    pub fn hz(&self) -> i64 {
        self.hz
    }
}

impl Rustics for TimeWindow {
    fn record_i64(&mut self, _sample: i64) {
        panic!("Rustics::TimeWindow:  i64 events are not permitted.");
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::TimeWindow:  f64 events are not permitted.");
    }

    fn record_event(&mut self) {
        let interval = (*self.timer).borrow_mut().finish();

        if interval > i64::MAX as u128 {
            panic!("TimeWindow::record_interval:  The interval is too long.");
        }

        self.integer_window.record_i64(interval as i64);
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.integer_window.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
        let interval = timer.finish();

        if interval > i64::MAX as u128 {
            panic!("TimeWindow::record_interval:  The interval is too long.");
        }

        self.integer_window.record_i64(interval as i64);
    }

    fn name(&self) -> String {
        self.integer_window.name()
    }

    fn title(&self) -> String {
        self.integer_window.title()
    }

    fn class(&self) -> &str {
        self.integer_window.class()
    }

    fn count(&self) ->u64 {
        self.integer_window.count()
    }

    fn log_mode(&self) -> isize {
        self.integer_window.log_mode()
    }

    fn mean(&self) ->f64 {
        self.integer_window.mean()
    }

    fn standard_deviation(&self) ->f64 {
        self.integer_window.standard_deviation()
    }

    fn variance(&self) ->f64 {
        self.integer_window.variance()
    }

    fn skewness(&self) ->f64 {
        self.integer_window.skewness()
    }

    fn kurtosis(&self) ->f64 {
        self.integer_window.kurtosis()
    }

    fn int_extremes(&self) -> bool {
        self.integer_window.int_extremes()
    }

    fn min_i64(&self) -> i64 {
        self.integer_window.min_i64()
    }

    fn min_f64(&self) -> f64 {
        self.integer_window.min_f64()
    }

    fn max_i64(&self) -> i64 {
        self.integer_window.max_i64()
    }

    fn max_f64(&self) -> f64 {
        self.integer_window.max_f64()
    }

    fn precompute(&mut self) {
        self.integer_window.precompute()
    }

    fn clear(&mut self) {
        self.integer_window.clear()
    }

    // Functions for printing

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
                &self.integer_window.title()
            };

        let crunched  = self.integer_window.crunch();

        let n         = self.integer_window.count();
        let min       = self.integer_window.min_i64();
        let max       = self.integer_window.max_i64();
        let log_mode  = self.integer_window.histo_log_mode();

        let mean      = crunched.mean;
        let variance  = compute_variance(n, crunched.moment_2);
        let skewness  = compute_skewness(n, crunched.moment_2, crunched.moment_3);
        let kurtosis  = compute_kurtosis(n, crunched.moment_2, crunched.moment_4);

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        let printer   = &mut *printer_box.lock().unwrap();

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);
        self.integer_window.print_histogram(printer);
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.integer_window.set_title(title)
    }

    fn set_id(&mut self, index: usize) {
        self.integer_window.set_id(index)
    }

    fn id(&self) -> usize {
        self.integer_window.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        self.integer_window.equals(other)
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        self.integer_window.histo_log_mode()
    }
}
