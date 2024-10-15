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
    fn print_lowest(&self);
            // Print the newest element of the lowest level

    fn print_lowest_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>);
            // Print a member of the statistics matrix

    fn print_all(&self, printer: Option<PrinterBox>, title: Option<&str>);
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

        // Create the initial statistics array and populate the first
        // statistics struct.

        let mut stats: Stats = Vec::with_capacity(dimensions[0].retention);

        for dimension in &dimensions {
            let window = Window::new(dimension.retention, dimension.period);

            stats.push(window);
        }

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
        // Push a new statistic if we've reached the event limit
        // for the current one.

        if
            self.auto_next != 0
        &&  self.event_count > 0
        &&  self.event_count % self.auto_next == 0 {
            self.advance();
        }

        let current = self.current_mut();

        current.record_i64(sample);
        self.event_count += 1;

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
        self.current().count()
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
        // Delete all the statistics that have been gathered.

        for level in &mut self.stats {
            level.clear();
        }

        // Push the initial statistics struct.

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

    fn print_lowest(&self) {
        self.local_print(HierIndex::new(0, 0), None, None);
    }

    fn print_lowest_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>) {
        self.local_print(index, printer, title);
    }

    // Print the statistics array.

    fn print_all(&self, printer: Option<PrinterBox>, title: Option<&str>) {
        let base_title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.len() {
                let title = format!("{}[{}][{}]", base_title, i, j);

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
        if self.stats.len() == 1 {
            return;
        }

        // Create the summary statistics struct.

        let exports  = self.exports(0);
        let new_stat = self.new_from_exports(&exports);

        self.stats[1].push(new_stat);

        // push a new statistics object onto level 0.

        self.stats[0].push(RunningInteger::new(&self.name));

        // Increment the advance op count.

        self.advance_count += 1;

        // Now move up the stack.

        let mut advance_point = 1;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hier_integer(name: &str, window_0_size: usize) -> HierInteger {
        let mut window_size = window_0_size;
        let mut dimensions  = Vec::<HierDimension>::with_capacity(3);

        for i in 0..3 {
            let dimension = HierDimension::new(window_size, 3 * window_size);

            dimensions.push(dimension);

            window_size *= (i + 1) * 4;
        }

        let descriptor = HierDescriptor::new(dimensions, Some(window_0_size));

        HierInteger::new(name, descriptor).unwrap()
    }

    fn test_simple_hier_integer() {
        let     window_size    = 4;
        let     signed_window  = window_size as i64;
        let     expected_count = window_size as u64;
        let mut events         = 0;
        let mut hier_integer   = make_hier_integer("hier test 1", window_size);

        assert!(hier_integer.stats[0].len() == 1);

        for i in 0..signed_window {
            hier_integer.record_i64(i);
            events += 1;
        }

        // We should not have pushed a new statistics object yet.

        assert!(hier_integer.stats[0].len() == 1);
        hier_integer.print();

        let expected_mean = (signed_window - 1) as f64 / 2.0;

        assert!(hier_integer.count()   == expected_count   );
        assert!(hier_integer.min_i64() == 0                );
        assert!(hier_integer.max_i64() == signed_window - 1);
        assert!(hier_integer.mean()    == expected_mean    );

        let mut sum = 0;

        for i in 0..signed_window {
            let value = signed_window + i;

            hier_integer.record_i64(value);
            sum    += value;
            events += 1;
        }


        assert!(hier_integer.stats[0].len() == 2);
        hier_integer.print();

        let floating_window = signed_window as f64;
        let expected_mean   = (sum as f64) / floating_window;

        assert!(hier_integer.count()   == expected_count       );
        assert!(hier_integer.min_i64() == signed_window        );
        assert!(hier_integer.max_i64() == 2 * signed_window - 1);
        assert!(hier_integer.mean()    == expected_mean        );

        let mut sum = 0;

        for i in 0..2 * signed_window {
            let value = -i;

            hier_integer.record_i64(value);

            if i >= signed_window {
                sum += value;
            }

            events += 1;
        }

        assert!(hier_integer.stats[0].len() == 4);
        hier_integer.print();

        let expected_mean = sum as f64 / floating_window;

        assert!(hier_integer.count()   == expected_count          );
        assert!(hier_integer.min_i64() == -(2 * signed_window - 1));
        assert!(hier_integer.max_i64() == -signed_window          );
        assert!(hier_integer.mean()    == expected_mean           );

        // Compute the size of level 1.  We advance level zero 1
        // time per window_size events, but the advance is delayed
        // until we actually record the event for the next window,
        // so we expect (events / window_size) - 1 since we record
        // events in multiples of the window size.

        let level_1_period = hier_integer.dimensions[1].period;
        let expected_len   = events / window_size;
        let expected_len   = expected_len - 1;

        assert!(hier_integer.stats[1].len() == expected_len);

        // Okay, compute the expected size at level 2.

        let expected_len = events / (window_size * level_1_period);

        assert!(hier_integer.stats[2].len() == expected_len);

    }

    #[test]
    fn run_tests() {
        println!("Running the hierarchical stats tests.");
        test_simple_hier_integer();
    }
}
