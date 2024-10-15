//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::any::Any;

use super::Rustics;
use super::window::Window;
use super::PrinterBox;
use super::RunningInteger;
use super::RunningImport;
use super::stdout_printer;
use super::sum_running;
use crate::TimerBox;

#[derive(Clone, Copy)]
pub struct HierDimension {
    period:        usize,
    retention:     usize,
}

impl HierDimension {
    pub fn new(period: usize, retention: usize) -> HierDimension {
        if retention < period {
            panic!("HierDimension::new:  The retention count must be at the period length.");
        }

        HierDimension { period, retention }
    }
}

#[derive(Clone)]
pub struct HierDescriptor {
    dimensions:  Vec<HierDimension>,
    auto_next:   usize,
}

impl HierDescriptor {
    pub fn new(dimensions: Vec<HierDimension>, auto_next: Option<usize>) -> HierDescriptor {
        let auto_next = auto_next.unwrap_or(0);

        HierDescriptor { dimensions, auto_next }
    }
}

pub struct HierIndex {
    level: usize,
    which: usize,
}

impl HierIndex {
    pub fn new(level: usize, which: usize) -> HierIndex {
        HierIndex { level, which }
    }
}

pub trait Hier {
    fn print(&self);
            // Print the newest element of the lowest level

    fn print_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>);
            // Print a member of the statistics matrix

    fn print_all(&self, printer: Option<PrinterBox>);
            // Print the entire statistics array.

    fn traverse(&mut self, traverser: &mut dyn HierTraverser);
            // Traverse the statistics.

    fn advance(&mut self);
            // sum the live elements of the given level into the next level up.
}

pub trait HierTraverser {
    fn visit(&mut self, member: &dyn Rustics);
}

type Stats = Vec<Window<RunningInteger>>;

pub struct HierInteger {
    name:          String,
    title:         String,
    id:            usize,
    dimensions:    Vec<HierDimension>,
    auto_next:     usize,
    event_count:   usize,
    advance_count: usize,

    stats:         Stats,

    printer:       PrinterBox,
}

impl HierInteger {
    pub fn new(name: &str, descriptor: HierDescriptor) -> Option<HierInteger> {
        let dimensions = descriptor.dimensions;

        if dimensions.is_empty() {
            return None;
        }

        for dimension in &dimensions {
            if dimension.retention < dimension.period {
                return None;
            }
        }

        let name          = name.to_string();
        let title         = name.clone();
        let id            = 0;
        let dimensions    = dimensions.to_vec();
        let auto_next     = descriptor.auto_next;
        let event_count   = 0;
        let advance_count = 0;
        let printer       = stdout_printer();

        // Create the initial statistics array.

        let mut stats: Stats = Vec::with_capacity(dimensions[0].retention);

        stats[0].push(RunningInteger::new(&name));

        let new = HierInteger {
            name,           title,      id,
            dimensions,     auto_next,  event_count,
            advance_count,  stats,      printer
        };

        Some(new)
    }

    pub fn current(&self) -> &RunningInteger {
        if self.stats[0].is_empty() {
            panic!("HierInteger:  The stats array is empty.");
        }

        let result =
            if let Some(result) = self.stats[0].newest() {
                result
            } else {
                panic!("HierInteger::current:  No data?");
            };

        result
    }

    pub fn current_mut(&mut self) -> &mut RunningInteger {
        if self.stats[0].is_empty() {
            panic!("HierInteger:  The stats array is empty.");
        }

        let result =
            if let Some(result) = self.stats[0].newest_mut() {
                result
            } else {
                panic!("HierInteger::current_mut:  No data?");
            };

        result
    }

    fn local_print(&self, index: HierIndex, printer_opt: Option<PrinterBox>, title_opt: Option<&str>) {
        let level = index.level;
        let which = index.which;

        let title =
            if let Some(title) = title_opt {
                title
            } else {
                &self.title
            };

        let title = format!("{}[{}][{}]", title, level, which);

        let printer_box =
            if let Some(printer) = printer_opt.clone() {
                printer.clone()
            } else {
                self.printer.clone()
            };

        let printer  = &mut *printer_box.lock().unwrap();

        if level >= self.stats.len() {
            printer.print(&title);
            printer.print(&format!("  This configuration has only {} levels.", self.stats.len()));
            return;
        }

        let target = self.stats[level].index_all(which);

        let target =
            if let Some(target) = target {
                target
            } else {
                printer.print(&title);
                printer.print(&format!("  That level has only {} entries.", self.stats[level].len()));
                return;
            };

        target.print_opts(printer_opt, title_opt);
    }

    fn exports(&self, level: usize) -> Vec<RunningImport> {
        let mut exports = Vec::<RunningImport>::new();
        let     level   = &self.stats[level];

        // Gather the statistics to sum.

        for stat in level.iter_live() {
            exports.push(stat.export());
        }

        exports
    }

    fn new_from_exports(&self, exports: &Vec<RunningImport>) -> RunningInteger {
        let name    = &self.name;
        let title   = &self.title;
        let printer = self.printer.clone();
        let sum     = sum_running(exports);

        RunningInteger::new_import(name, title, printer, sum)
    }
}

impl Rustics for HierInteger {
    fn record_i64(&mut self, sample: i64) {
        let current = self.current_mut();

        current.record_i64(sample);

        // Push a new statistic if we've reached the event limit
        // for the current one.

        if
            self.auto_next != 0
        &&  self.stats.len() > 1
        &&  self.event_count % self.auto_next == 0 {
            self.advance();
        }
    }

    fn record_f64(&mut self, _sample: f64) {
        panic!("HierInteger:  record_f64 is not supported.");
    }

    fn record_event(&mut self) {
        panic!("HierInteger:  record_event is not supported.");
    }

    fn record_time(&mut self, _sample: i64) {
        panic!("HierInteger:  record_time is not supported.");
    }

    fn record_interval(&mut self, _timer: &mut TimerBox) {
        panic!("HierInteger:  record_integer is not supported.");
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
        panic!("HierRuningInteger:  not supported");
    }

    fn log_mode(&self) -> isize {
        self.current().log_mode()
    }

    fn mean(&self) -> f64 {
        self.current().mean()
    }

    fn standard_deviation(&self) -> f64 {
        self.current().standard_deviation()
    }

    fn variance(&self) -> f64 {
        self.current().variance()
    }

    fn skewness(&self) -> f64 {
        self.current().skewness()
    }

    fn kurtosis(&self) -> f64 {
        self.current().kurtosis()
    }

    fn int_extremes(&self) -> bool {
        true
    }

    fn min_i64(&self) -> i64 {
        self.current().min_i64()
    }

    fn min_f64(&self) -> f64 {
        panic!("Hier:  min_f64 is not implemented.");
    }

    fn max_i64(&self) -> i64 {
        self.current().max_i64()
    }

    fn max_f64(&self) -> f64 {
        panic!("Hier:  max_f64 is not implemented.");
    }


    fn precompute(&mut self) {
        self.current_mut().precompute();
    }

    fn clear(&mut self) {
        for level in &mut self.stats {
            level.clear();
        }

        self.stats[0].push(RunningInteger::new(&self.name));
    }

    // Functions for printing
    //   print          prints the current statistic bucket
    //
    //   print_opts     prints the current statistics bucket with the options
    //                      specified

    fn print(&self) {
        self.current().print_opts(None, None);
    }

    fn print_opts(&self, printer: Option<PrinterBox>, title: Option<&str>) {
        self.local_print(HierIndex::new(0, 0), printer, title);
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    // For internal use only.
    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<HierInteger>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        self.current().histo_log_mode()
    }

    fn to_running_integer(&mut self) -> Option<&mut RunningInteger> {
        None
    }
}

impl Hier for HierInteger {
    // Print a member of the statistics matrix

    fn print(&self) {
        self.local_print(HierIndex::new(0, 0), None, None);
    }

    fn print_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>) {
        self.local_print(index, printer, title);
    }

    // Print the statistics array.

    fn print_all(&self, printer: Option<PrinterBox>) {
        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.len() {
                let title = format!("{}[{}][{}]", self.title, i, j);

                let stat =
                    if let Some(element) = self.stats[i].index_all(j) {
                        element
                    } else {
                        panic!("HierInteger::print_all:  The index_all failed.");
                    };

                stat.print_opts(printer.clone(), Some(&title));
            }
        }
    }

    // Traverse the live statistics.

    fn traverse(&mut self, traverser: &mut dyn HierTraverser) {
        for level in &self.stats {
            for i in 0..level.live_len() {
                if let Some(stat) = level.index_live(i) {
                    traverser.visit(stat);
                } else {
                    panic!("HierInteger::traverse:  Index {} failed.", i)
                }
            }
        }
    }

    // Sum the live statistics at the given level and push the sum
    // onto the higher level.  That level might need to be summed,
    // as well.

    fn advance(&mut self) {
        // Create the summary statistics struct.

        let exports  = self.exports(0);
        let new_stat = self.new_from_exports(&exports);

        self.stats[0].push(new_stat);

        self.advance_count += 1;

        let mut advance_point = self.dimensions[0].period;

        for i in 1..self.dimensions.len() - 1 {
            advance_point *= self.dimensions[i].period;

            if self.advance_count % advance_point == 0 {
                let exports  = self.exports(i);
                let new_stat = self.new_from_exports(&exports);

                self.stats[i + 1].push(new_stat);
            } else {
                break;
            }
        }
    }
}
