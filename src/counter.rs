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
use super::stdout_printer;
use super::printable::Printable;

pub struct Counter {
    name:       String,
    title:      String,
    count:      i64,
    id:         usize,
    printer:    PrinterBox,
}

impl Counter {
    pub fn new(name: &str, printer: PrinterOption) -> Counter {
        let name    = String::from(name);
        let title   = name.clone();
        let count   = 0;
        let id      = 0;

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        Counter { name, title, count, id, printer }
    }
}

impl Rustics for Counter {
    fn record_i64(&mut self, sample: i64) {
        if sample < 0 {
            panic!("Counter::record_i64:  The sample is negative.");
        }

        self.count += sample;
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Counter::record_f64:  not supported");
    }

    fn record_event(&mut self) {
        self.count += 1;
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("Counter::record_time:  not supported");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("Counter::record_interval:  not supported");
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
        self.count as u64
    }

    fn log_mode(&self) -> isize {
        panic!("Counter::log_mode:  not supported");
    }

    fn mean(&self) -> f64 {
        panic!("Counter::mean:  not supported");
    }

    fn standard_deviation(&self) -> f64 {
        panic!("Counter::standard_deviation:  not supported");
    }

    fn variance(&self) -> f64 {
        panic!("Counter::variance:  not supported");
    }

    fn skewness(&self) -> f64 {
        panic!("Counter::skewness:  not supported");
    }

    fn kurtosis(&self) -> f64 {
        panic!("Counter::kurtosis:  not supported");
    }

    fn int_extremes(&self) -> bool {
        false
    }

    fn min_i64(&self) -> i64 {
        panic!("Counter::min_i64:  not supported");
    }

    fn min_f64(&self) -> f64 {
        panic!("Counter::min_f64:  not supported");
    }

    fn max_i64(&self) -> i64 {
        panic!("Counter::max_i64:  not supported");
    }

    fn max_f64(&self) -> f64 {
        panic!("Counter::max_f64:  not supported");
    }

    fn precompute(&mut self) {
    }

    fn clear(&mut self) {
        self.count = 0;
    }

    // Functions for printing:

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

        let printer = &mut *printer_box.lock().unwrap();

        printer.print(title);
        Printable::print_integer("Count", self.count, printer);
    }

    // For internal use only.
    fn set_title(&mut self, title: &str) {
        self.title = String::from(title);
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<Counter>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        panic!("Counter::histo_log_mode:  not supported");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_simple_counter() {
        let test_limit  = 20;
        let mut counter = Counter::new("test counter", None);

        for i in 1..test_limit + 1 {
            counter.record_event();
            counter.record_i64(i);
        }

        let events   = test_limit;
        let sequence = ((test_limit + 1) * test_limit) / 2;
        let expected = events + sequence;

        assert!(counter.count() == expected as u64);

        counter.print();
    }

    #[test]
    fn run_tests() {
        test_simple_counter();
    }
}
