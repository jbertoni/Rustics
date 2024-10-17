//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::any::Any;

use super::Rustics;
use super::Histogram;
use super::PrinterBox;
use super::PrinterOption;
use super::stdout_printer;
use super::TimerBox;
use super::timer_box_hz;
use super::printable::Printable;
use super::running_integer::RunningInteger;

#[derive(Clone)]
pub struct RunningTime {
    printer:            PrinterBox,

    running_integer:    Box<RunningInteger>,
    timer:              TimerBox,
    hz:                 i64,
}

impl RunningTime {
    pub fn new(name_in: &str, timer: TimerBox) -> RunningTime {
        let hz = timer_box_hz(&timer);

        if hz > i64::MAX as u128 {
            panic!("Rustics::RunningTime:  The timer hz value is too large.");
        }

        let hz              = hz as i64;
        let printer         = stdout_printer();
        let running_integer = Box::new(RunningInteger::new(name_in, Some(printer)));
        let printer         = stdout_printer();

        RunningTime { printer, running_integer, timer, hz }
    }

    pub fn hz(&self) -> i64 {
        self.hz
    }
}

impl Rustics for RunningTime {
    fn record_i64(&mut self, _sample: i64) {
        panic!("Rustics::RunningTime:  i64 events are not permitted.");
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Rustics::RunningTime:  f64 events are not permitted.");
    }

    fn record_event(&mut self) {
        let mut timer    = (*self.timer).borrow_mut();
        let     interval = timer.finish();  // read and restart the timer

        if interval > i64::MAX as u128 {
            panic!("RunningTime::record_interval:  The interval is too long.");
        }

        self.running_integer.record_i64(interval as i64);
    }

    fn record_time(&mut self, sample: i64) {
        assert!(sample >= 0);
        self.running_integer.record_i64(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        let mut timer = (*timer).borrow_mut();
        let interval = timer.finish();

        if interval > i64::MAX as u128 {
            panic!("RunningTime::record_interval:  The interval is too long.");
        }

        self.running_integer.record_i64(interval as i64);
    }

    fn name(&self) -> String {
        self.running_integer.name()
    }

    fn title(&self) -> String {
        self.running_integer.title()
    }

    fn class(&self) -> &str {
        self.running_integer.class()
    }

    fn count(&self) ->u64 {
        self.running_integer.count()
    }

    fn log_mode(&self) -> isize {
        self.running_integer.log_mode()
    }

    fn mean(&self) ->f64 {
        self.running_integer.mean()
    }

    fn standard_deviation(&self) ->f64 {
        self.running_integer.standard_deviation()
    }

    fn variance(&self) ->f64 {
        self.running_integer.variance()
    }

    fn skewness(&self) ->f64 {
        self.running_integer.skewness()
    }

    fn kurtosis(&self) ->f64 {
        self.running_integer.kurtosis()
    }

    fn int_extremes(&self) -> bool {
        self.running_integer.int_extremes()
    }

    fn min_i64(&self) -> i64 {
        self.running_integer.min_i64()
    }

    fn min_f64(&self) -> f64 {
        self.running_integer.min_f64()
    }

    fn max_i64(&self) -> i64 {
        self.running_integer.max_i64()
    }

    fn max_f64(&self) -> f64 {
        self.running_integer.max_f64()
    }

    fn precompute(&mut self) {
        self.running_integer.precompute()
    }

    fn clear(&mut self) {
        self.running_integer.clear()
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
                &self.running_integer.title()
            };

        let printer  = &mut *printer_box.lock().unwrap();
        let n        = self.count();
        let min      = self.min_i64();
        let max      = self.max_i64();
        let log_mode = self.running_integer.histo_log_mode();
        let mean     = self.mean();
        let variance = self.variance();
        let skewness = self.skewness();
        let kurtosis = self.kurtosis();

        let printable = Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        printer.print(title);
        printable.print_common_integer_times(self.hz, printer);
        printable.print_common_float_times(self.hz, printer);

        self.running_integer.print_histogram();
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.running_integer.set_title(title);
    }

    fn set_id(&mut self, index: usize) {
        self.running_integer.set_id(index)
    }

    fn id(&self) -> usize {
        self.running_integer.id()
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        self.running_integer.equals(other)
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        self.running_integer.histo_log_mode()
    }
}
