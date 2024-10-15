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
    set:   HierSet,
    level: usize,
    which: usize,
}

pub enum HierSet {
    All,
    Live,
}

impl HierIndex {
    pub fn new(set: HierSet, level: usize, which: usize) -> HierIndex {
        HierIndex { set, level, which }
    }
}

pub trait Hier {
    fn print_index_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>);
            // Print a member of the statistics matrix

    fn print_all(&self, printer: Option<PrinterBox>, title: Option<&str>);
            // Print the entire statistics array.

    fn traverse_live(&mut self, traverser: &mut dyn HierTraverser);
            // Traverse the statistics.

    fn advance(&mut self);
            // sum the live elements of the given level into the next level up.

    fn live_len(&self, level: usize) -> usize;
    fn all_len (&self, level: usize) -> usize;
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

        let set =
            match index.set {
                HierSet::Live => { "live" }
                HierSet::All  => { "all"  }
            };

        let title = format!("{}[{}].{}[{}]", title, level, set, which);

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


        let target =
            match index.set {
                HierSet::Live => { self.stats[level].index_live(which) }
                HierSet::All  => { self.stats[level].index_all (which) }
            };

        let target =
            if let Some(target) = target {
                target
            } else {
                printer.print(&title);
                printer.print("  That index is out of bounds.");
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
        // for the current one.  Do this before push the next
        // event so that users see an empty current statistic only
        // before recording any events at all.
        //

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
        let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

        self.local_print(index, None, None);
    }

    fn print_opts(&self, printer: Option<PrinterBox>, title: Option<&str>) {
        let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

        self.local_print(index, printer, title);
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

    fn print_index_opts(&self, index: HierIndex, printer: Option<PrinterBox>, title: Option<&str>) {
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

            for j in 0..level.all_len() {
                let title = format!("{}[{}].all[{}]", base_title, i, j);

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

    fn traverse_live(&mut self, traverser: &mut dyn HierTraverser) {
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

        // Increment the advance op count.

        self.advance_count += 1;

        // Now move up the stack.

        let mut advance_point = 1;

        for i in 0..self.dimensions.len() - 1 {
            advance_point *= self.dimensions[i].period;

            if self.advance_count % advance_point == 0 {
                let exports  = self.exports(i);
                let new_stat = self.new_from_exports(&exports);

                self.stats[i + 1].push(new_stat);
            } else {
                break;
            }
        }

        // Create the summary statistics struct and push it onto the
        // level zero stack.

        self.stats[0].push(RunningInteger::new(&self.name));
    }

    fn all_len(&self, level: usize) -> usize {
        self.stats[level].all_len()
    }

    fn live_len(&self, level: usize) -> usize {
        self.stats[level].live_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hier_integer(name: &str, level_0_period: usize, auto_next: usize) -> HierInteger {
        let     levels      = 4;
        let     dimension   = HierDimension::new(level_0_period, 3 * level_0_period);
        let mut dimensions  = Vec::<HierDimension>::with_capacity(levels);

        // Push the level 0 descriptor.

        dimensions.push(dimension);

        // Create a hierarchy.

        let mut period = 4;

        for _i in 1..levels {
            let dimension = HierDimension::new(period, 3 * period);

            dimensions.push(dimension);

            period += 2;
        }

        let descriptor = HierDescriptor::new(dimensions, Some(auto_next));

        HierInteger::new(name, descriptor).unwrap()
    }

    fn compute_events_per_entry(hier_integer: &HierInteger, level: usize) -> i64 {
        let mut result = hier_integer.auto_next as i64;

        assert!(result > 0);

        for i in 0..level {
            result *= hier_integer.dimensions[i].period as i64;
        }

        result
    }

    fn roundup(value: i64, multiple: i64) -> i64 {
        (((value + multiple - 1) / multiple)) * multiple
    }

    fn compute_len(hier_integer: &HierInteger, level: usize, set: HierSet, events: i64) -> usize {
        assert!(events > 0);

        let recorded_events =
            if level == 0 {
                let auto_next = hier_integer.auto_next as i64;
                
                roundup(events, auto_next)
            } else {
                events - 1
            };

        let     events_per_entry = compute_events_per_entry(&hier_integer, level);
        let     pushes           = recorded_events / events_per_entry;
        let     period           = hier_integer.dimensions[level].period as i64;
        let     size_limit       = hier_integer.dimensions[level].retention as i64;

        let mut length = 
            match set {
                HierSet::Live => { std::cmp::min(pushes, period    ) }
                HierSet::All  => { std::cmp::min(pushes, size_limit) }
            };

        if length == 0 && level == 0 {
            length = 1;
        }

        length as usize
    }

    fn check_sizes(hier_integer: &HierInteger, events: i64, verbose: bool) {
        for level in 0..hier_integer.stats.len() {
            let expected_all_len  = compute_len(hier_integer, level, HierSet::All,  events);
            let expected_live_len = compute_len(hier_integer, level, HierSet::Live, events);

            let actual_all_len    = hier_integer.stats[level].all_len();
            let actual_live_len   = hier_integer.stats[level].live_len();

            if verbose {
                println!("check_sizes:  at level {}, events {}", level, events);

                println!("    expected_all {}, expected_live {}",
                    expected_all_len, expected_live_len);

                println!("    actual_all {}, actual_live {}",
                    actual_all_len, actual_live_len);
            }

            assert!(actual_all_len  == expected_all_len );
            assert!(actual_live_len == expected_live_len);
        }
    }

    // This is a fairly straightforward test that just pushes a lot of
    // values into a HierInteger struct.  It is long because it takes a
    // fair number of operations to force higer-level statistics into
    // use.

    fn simple_hier_test() {
        let     auto_next      = 4;
        let     level_0_period = 4;
        let     signed_auto    = auto_next as i64;
        let mut events         = 0;
        let mut sum_of_events  = 0;

        let mut hier_integer   = make_hier_integer("hier test 1", level_0_period, auto_next);

        // Check that the struct matches our expectations.

        assert!(auto_next == hier_integer.auto_next);
        assert!(hier_integer.stats[0].all_len() == 1);
        assert!(hier_integer.dimensions[0].period == level_0_period);

        let expected_count = auto_next as u64;

        for i in 0..signed_auto {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            if i < signed_auto - 1 {
                let mean  = sum_of_events as f64 / (i + 1) as f64;

                let all_len_0 = hier_integer.stats[0].all_len();
                let count     = (i + 1) as u64;

                assert!(all_len_0              == 1    );
                assert!(hier_integer.count()   == count);
                assert!(hier_integer.min_i64() == 0    );
                assert!(hier_integer.max_i64() == i    );
                assert!(hier_integer.mean()    == mean );

                check_sizes(&hier_integer, events, false);
            }
        }

        let mean = sum_of_events as f64 / events as f64;

        assert!(hier_integer.count() as i64 == signed_auto    );
        assert!(hier_integer.min_i64()      == 0              );
        assert!(hier_integer.max_i64()      == signed_auto - 1);
        assert!(hier_integer.mean()         == mean           );

        check_sizes(&hier_integer, events, false);
        hier_integer.print();

        assert!(hier_integer.count() == events as u64);

        let mut sum = 0;

        for i in 0..signed_auto {
            let value = signed_auto + i;

            hier_integer.record_i64(value);
            sum           += value;
            events        += 1;
            sum_of_events += value;

            check_sizes(&hier_integer, events, false);
        }


        assert!(hier_integer.stats[0].all_len() == 2);
        hier_integer.print();

        let floating_window = signed_auto as f64;
        let expected_mean   = (sum as f64) / floating_window;

        assert!(hier_integer.count()   == expected_count     );
        assert!(hier_integer.min_i64() == signed_auto        );
        assert!(hier_integer.max_i64() == 2 * signed_auto - 1);
        assert!(hier_integer.mean()    == expected_mean      );

        let mut sum = 0;

        for i in 0..2 * signed_auto {
            let value = -i;

            hier_integer.record_i64(value);

            if i >= signed_auto {
                sum += value;
            }

            events        += 1;
            sum_of_events += value;

            check_sizes(&hier_integer, events, false);
        }

        hier_integer.print();

        let expected_mean = sum as f64 / floating_window;

        assert!(hier_integer.count()   == expected_count        );
        assert!(hier_integer.min_i64() == -(2 * signed_auto - 1));
        assert!(hier_integer.max_i64() == -signed_auto          );
        assert!(hier_integer.mean()    == expected_mean         );

        // Now force a level 1 stat object.

        for i in 0..(auto_next * level_0_period) as i64 {
            hier_integer.record_i64(i);
            hier_integer.record_i64(-i);

            events += 2;
            // sum_of_events += i + -i;

            check_sizes(&hier_integer, events, false);
        }

        let expected_len = compute_len(&hier_integer, 1, HierSet::All,  events);
        let actual_len   = hier_integer.stats[1].all_len();

        assert!(expected_len > 0);
        assert!(actual_len == expected_len);

        for i in 0..(auto_next * level_0_period / 2) as i64 {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            check_sizes(&hier_integer, events, false);
        }

        for i in 0..(auto_next * level_0_period / 2) as i64 {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            check_sizes(&hier_integer, events, false);
        }

        // Compute the expected mean once we force level 0 to
        // be summed.

        let expected_mean = sum_of_events as f64 / events as f64;

        // Force the next push from level 0 by recording an event.
        // This should produce a level 2 entry.

        let value = 0;

        hier_integer.record_i64(value);

        events        += 1;
        sum_of_events += value;

        let expected_len = compute_len(&hier_integer, 2, HierSet::All, events);

        assert!(expected_len == 1);

        // Check the length.  Use a hardcode value, too, to check the
        // sanity of the preceding code.

        assert!(hier_integer.stats[2].all_len() == expected_len);

        let stat = hier_integer.stats[2].newest().unwrap();

        stat.print();
        hier_integer.print_all(None, None);

        assert!(stat.mean() == expected_mean);

        println!("simple_hier_test:  {} events, sum {}", events, sum_of_events);
    }

    struct TestTraverser {
        count:  i64,
    }

    impl TestTraverser {
        pub fn new() -> TestTraverser {
            let count = 0;

            TestTraverser { count }
        }
    }

    impl HierTraverser for TestTraverser {
        fn visit(&mut self, _member: &dyn Rustics) {
            self.count += 1;
        }
    }

    // Shove enough events into the stat to get a level 3 entry.  Check that
    // the count and the mean of the level 3 stat match our expectations.

    fn long_test() {
        let     auto_next      = 2;
        let     level_0_period = 4;
        let mut hier_integer   = make_hier_integer("long test", level_0_period, auto_next);
        let mut events         = 0;

        while hier_integer.stats[3].all_len()  == 0 {
            events += 1;
            hier_integer.record_i64(events);

            check_sizes(&hier_integer, events, false);
        }

        let     dimensions         = &hier_integer.dimensions;
        let mut events_per_level_3 = auto_next;

        for i in 0..dimensions.len() - 1 {
            events_per_level_3 *= dimensions[i].period;
        }

        let stat = hier_integer.stats[3].newest().unwrap();

        let events_in_stat = (events - 1) as f64;
        let sum            = (events_in_stat * (events_in_stat + 1.0)) / 2.0;
        let mean           = sum / events_in_stat;

        println!("long_test:  stats.mean() {}, expected {}", stat.mean(), mean);

        assert!(stat.count() as i64 == events - 1               );
        assert!(stat.count() as i64 == events_per_level_3 as i64);
        assert!(stat.mean()         == mean                     );

        hier_integer.print_all(None, None);

        // Do a quick test of the traverser.

        let mut traverser = TestTraverser::new();
        let mut predicted = 0;

        hier_integer.traverse_live(&mut traverser);

        for level in 0..hier_integer.dimensions.len() {
            predicted += hier_integer.live_len(level) as i64;
        }

        println!("long_test:  traversed {} stats structs, predicted {}",
            traverser.count, predicted);
        assert!(traverser.count == predicted);
    }

    #[test]
    fn run_tests() {
        println!("Running the hierarchical stats tests.");
        simple_hier_test();
        long_test();
    }
}
