//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!
//! ## Type
//!
//! * Counter
//!     * Counter implements a simple i64 counter that can be printed
//!       with other Rustics instances.
//!
//!     * It is intended to be used for counting events or summing values
//!       for which summary statistics are not wanted.
//!
//! ## Example
//!```
//!     use rustics::Rustics;
//!     use rustics::counter::Counter;
//!     use rustics::PrintOpts;
//!     use rustics::Units;
//!
//!     // Create a count and pretend that it's counting bytes.  The title
//!     // defaults to the name, and the output is to stdout, and those
//!     // are fine for this example.
//!
//!     let test_limit  = 20;
//!     let singular    = "byte".to_string();
//!     let plural      = "bytes".to_string();
//!     let units       = Some(Units { singular, plural });
//!     let printer     = None;
//!     let title       = None;
//!     let histo_opts  = None;
//!     let print_opts  = Some(PrintOpts { printer, title, units, histo_opts });
//!     let mut counter = Counter::new("test counter", &print_opts);
//!
//!     // Add some counts to the counter.  record_event() adds one, to
//!     // implement an event counter.  record_i64() adds any i64 value
//!     // to the counter, for keeping a sum when statistics like the
//!     // mean aren't useful.
//!
//!     for i in 1..=test_limit {
//!         counter.record_event();
//!         counter.record_i64(i);
//!     }
//!
//!     // Now compute what we expect as the total count, and check that
//!     // against the counter's value.  The sum of the sequence
//!     //     1 + 2 + 3 ... + n 
//!     // is
//!     //     (n * (n + 1)) / 2
//!
//!     let events   = test_limit;
//!     let sequence = ((test_limit + 1) * test_limit) / 2;
//!     let expected = events + sequence;
//!
//!     assert!(counter.count() == expected as u64);
//!
//!     counter.print();
//!```

use std::any::Any;

use super::Rustics;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use super::ExportStats;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::Units;
use super::TimerBox;
use super::printable::Printable;
use super::parse_print_opts;

/// The Counter type provides a simple counter that implements
/// the Rustics trait.

#[derive(Clone)]
pub struct Counter {
    name:       String,
    title:      String,
    count:      i64,
    id:         usize,
    printer:    PrinterBox,
    units:      Units,
}

impl Counter {
    /// Constructs an instance with a given name and optional Printer
    /// instance

    pub fn new(name: &str, print_opts: &PrintOption) -> Counter {
        let (printer, title, units, _histo_opts) = parse_print_opts(print_opts, name);

        let name    = String::from(name);
        let count   = 0;
        let id      = usize::MAX;


        Counter { name, count, id, printer, title, units }
    }

    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    fn event_increment(&self) -> i64 {
        1
    }
}

impl Rustics for Counter {
    /// Adds a value to the counter.

    fn record_i64(&mut self, sample: i64) {
        if sample < 0 {
            panic!("Counter::record_i64:  The sample is negative.");
        }

        self.count += sample;
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("Counter::record_f64:  not supported");
    }

    /// Increments the counter by one.

    fn record_event(&mut self) {
        self.count += self.event_increment();
    }

    fn record_event_report(&mut self) -> i64 {
        self.count += self.event_increment();
        self.event_increment()
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

    fn float_extremes(&self) -> bool {
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
        Printable::print_integer_units("Count", self.count, printer, &self.units);
        printer.print("");
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

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        None
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        None
    }

    fn export_stats(&self) -> ExportStats {
        let n          = self.count as u64;
        let nans       = 0;
        let infinities = 0;
        let min_i64    = i64::MIN;
        let max_i64    = i64::MAX;
        let min_f64    = f64::MIN;
        let max_f64    = f64::MAX;
        let log_mode   = 0;
        let mode_value = 0.0;
        let mean       = 0.0;
        let variance   = 0.0;
        let skewness   = 0.0;
        let kurtosis   = 0.0;
        let units      = self.units.clone();

        let printable =
            Printable { 
                n,           nans,      infinities,  min_i64,   max_i64,   min_f64,
                max_f64,     log_mode,  mean,        variance,  skewness,  kurtosis,
                mode_value,  units
            };

        let log_histogram   = None;
        let float_histogram = None;

        ExportStats { printable, log_histogram, float_histogram }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_simple_counter() {
        let test_limit  = 20;
        let mut counter = Counter::new("test counter", &None);

        for i in 1..=test_limit {
            counter.record_event();
            counter.record_i64(i);
        }

        // Now compute what we expect as the total count, and
        // check that against the counter's value.  record_event
        // increments by one.

        let events   = test_limit;
        let sequence = ((test_limit + 1) * test_limit) / 2;
        let expected = events + sequence;

        assert!(counter.count() == expected as u64);

        assert!(!counter.float_extremes());
        assert!(!counter.int_extremes  ());


        counter.print();
    }

    #[test]
    fn run_tests() {
        test_simple_counter();
    }
}
