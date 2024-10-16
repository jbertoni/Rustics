//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use super::Rustics;
use super::PrinterBox;
use super::PrinterOption;
use super::TimerBox;
use super::window::Window;
use std::cell::RefCell;
use std::rc::Rc;
use std::any::Any;

pub type MemberRc    = Rc<RefCell<dyn HierMember   >>;
pub type GeneratorRc = Rc<RefCell<dyn HierGenerator>>;
pub type ExporterRc  = Rc<RefCell<dyn HierExporter >>;

#[derive(Clone)]
pub struct HierDescriptor {
    dimensions:     Vec<HierDimension>,
    auto_next:      i64,
}

impl HierDescriptor {
    pub fn new(dimensions: Vec<HierDimension>, auto_next: Option<i64>) -> HierDescriptor {
        let auto_next = auto_next.unwrap_or(0);

        if auto_next < 0 {
            panic!("HierDescriptor::new:  The auto_next value can't be negative.");
        }

        HierDescriptor { dimensions, auto_next }
    }
}

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

#[derive(Clone, Copy)]
pub struct HierIndex {
    set:   HierSet,
    level: usize,
    which: usize,
}

#[derive(Clone, Copy)]
pub enum HierSet {
    All,
    Live,
}

impl HierIndex {
    pub fn new(set: HierSet, level: usize, which: usize) -> HierIndex {
        HierIndex { set, level, which }
    }
}

pub trait HierExporter {
    fn as_any    (&self)      -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait HierTraverser {
    fn visit(&mut self, member: &mut dyn Rustics);
}

pub trait HierGenerator {
    fn make_from_exporter(&self, name: &str, printer: PrinterBox, exports: ExporterRc) -> MemberRc;

    fn make_member       (&self, name: &str, printer: PrinterBox)  -> MemberRc;
    fn make_exporter     (&self)                                   -> ExporterRc;

    fn push              (&self, exports: ExporterRc, member: MemberRc);
}

pub trait HierMember {
    fn to_rustics    (&self    ) -> &dyn Rustics;
    fn to_rustics_mut(&mut self) -> &mut dyn Rustics;
    fn as_any        (&self    ) -> &dyn Any;
    fn as_any_mut    (&mut self) -> &mut dyn Any;
}

pub struct Hier {
    dimensions:     Vec<HierDimension>,
    generator:      GeneratorRc,
    stats:          Vec<Window<MemberRc>>,
    name:           String,
    title:          String,
    id:             usize,
    class:          String,
    auto_next:      i64,
    advance_count:  i64,
    event_count:    i64,
    printer:        PrinterBox,
}

pub struct HierConfig {
    pub descriptor:  HierDescriptor,
    pub generator:   GeneratorRc,
    pub class:       String,
    pub name:        String,
    pub title:       String,
    pub printer:     PrinterBox,
}

impl Hier {
    pub fn new(configuration: HierConfig) -> Hier {
        let     descriptor    = configuration.descriptor;
        let     generator     = configuration.generator;
        let     name          = configuration.name;
        let     title         = configuration.title;
        let     printer       = configuration.printer;
        let     class         = configuration.class;

        let     auto_next     = descriptor.auto_next;
        let     dimensions    = descriptor.dimensions;
        let     id            = 0;
        let mut stats         = Vec::with_capacity(dimensions.len());
        let     advance_count = 0;
        let     event_count   = 0;

        for dimension in &dimensions {
            stats.push(Window::new(dimension.retention, dimension.period));
        }

        let member = generator.borrow_mut().make_member(&name, printer.clone());

        stats[0].push(member);

        Hier {
            dimensions,   generator,  stats,
            name,         title,      id,
            class,        auto_next,  advance_count,
            event_count,  printer
        }
    }

    pub fn current(&self) -> MemberRc {
        let member = self.stats[0].newest().unwrap();

        member.clone()
    }

    pub fn print_index_opts(&self, index: HierIndex, printer: PrinterOption, title: Option<&str>) {
        self.local_print(index, printer, title);
    }

    pub fn print_all(&self, printer: PrinterOption, title: Option<&str>) {
        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.all_len() {
                let index = HierIndex::new(HierSet::All, i, j);

                self.local_print(index, printer.clone(), title)
            }
        }
    }

    pub fn clear_all(&mut self) {
        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.all_len() {
                let     target = self.stats[i].index_all(j);
                let mut target = target.unwrap().borrow_mut();

                target.to_rustics_mut().clear();
            }
        }
    }

    pub fn traverse_live(&mut self, traverser: &mut dyn HierTraverser) {
        for level in &mut self.stats {
            for member in level.iter_live() {
                let mut borrow  = member.borrow_mut();
                let     rustics = borrow.to_rustics_mut();

                traverser.visit(rustics);
            }
        }
    }

    pub fn advance(&mut self) {
        if self.stats.len() == 1 {
            let generator = self.generator.borrow();
            let member    = generator.make_member(&self.name, self.printer.clone());

            self.stats[0].push(member);
            return;
        }

        // Increment the advance op count.

        self.advance_count += 1;

        // Now move up the stack.

        let mut advance_point = 1;
        let     generator     = self.generator.borrow();

        for i in 0..self.dimensions.len() - 1 {
            advance_point *= self.dimensions[i].period as i64;

            if self.advance_count % advance_point == 0 {
                let exporter = self.make_exporter(i);
                let new_stat = generator.make_from_exporter(&self.name, self.printer.clone(), exporter);

                self.stats[i + 1].push(new_stat);
            } else {
                break;
            }
        }

        // Create the summary statistics struct and push it onto the
        // level zero stack.

        let member = generator.make_member(&self.name, self.printer.clone());

        self.stats[0].push(member);
    }

    pub fn live_len(&self, level: usize) -> usize {
        self.stats[level].live_len()
    }

    pub fn all_len(&self, level: usize) -> usize {
        self.stats[level].all_len()
    }

    pub fn event_count(&self) -> i64 {
        self.event_count
    }

    fn local_print(&self, index: HierIndex, printer_opt: PrinterOption, title_opt: Option<&str>) {
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

        if level >= self.stats.len() {
            let printer = &mut *printer_box.lock().unwrap();
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
                let printer = &mut *printer_box.lock().unwrap();
                printer.print(&title);
                printer.print("  That index is out of bounds.");
                return;
            };

        let target = target.borrow();
        target.to_rustics().print_opts(printer_opt, title_opt);
    }

    fn make_exporter(&self, level: usize) -> ExporterRc {
        let generator    = self.generator.borrow();
        let exporter_rc  = generator.make_exporter();

        let level = &self.stats[level];

        // Gather the statistics to sum.

        for stat in level.iter_live() {
            generator.push(exporter_rc.clone(), stat.clone());
        }

        exporter_rc
    }

    fn check_advance(&mut self) {
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
        
        // advance the event count...

        self.event_count += 1;
    }
}

impl Rustics for Hier {
    fn record_i64(&mut self, value: i64) {
        self.check_advance();

        let     member  = self.stats[0].newest_mut().unwrap();
        let mut borrow  = member.borrow_mut();

        let rustics = borrow.to_rustics_mut();

        rustics.record_i64(value)
    }

    fn record_f64(&mut self, sample: f64) {
        self.check_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_f64(sample);
    }

    fn record_event(&mut self) {
        self.record_i64(1);
    }

    fn record_time(&mut self, sample: i64) {
        self.check_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_time(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        self.check_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_interval(timer);
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        &self.class
    }

    fn count(&self) -> u64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.count()
    }

    fn log_mode(&self) -> isize {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.log_mode()
    }

    fn mean(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.mean()
    }

    fn standard_deviation(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.standard_deviation()
    }

    fn variance(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.variance()
    }

    fn skewness(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.skewness()
    }

    fn kurtosis(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.kurtosis()
    }

    fn int_extremes(&self) -> bool {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.int_extremes()
    }

    fn min_i64(&self) -> i64  {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.min_i64()
    }

    fn min_f64(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.min_f64()
    }

    fn max_i64(&self) -> i64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.max_i64()
    }

    fn max_f64(&self) -> f64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.max_f64()
    }

    fn precompute(&mut self) {
        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.precompute();
    }

    fn clear(&mut self) {
        self.clear_all();
    }

    // Functions for printing

    fn print(&self) {
        let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

        self.local_print(index, None, None);
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.print_opts(printer, title);
    }

    fn set_title(&mut self, title: &str) {
        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.set_title(title);
    }

    fn set_id(&mut self, index: usize) {
        self.id = index;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.equals(other)
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histo_log_mode(&self) -> i64 {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.histo_log_mode()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::stdout_printer;
    use crate::RunningGenerator;

    pub fn make_hier(generator: GeneratorRc, level_0_period: usize, auto_next: usize) -> Hier {
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

        let auto_next     = Some(auto_next as i64);
        let descriptor    = HierDescriptor::new(dimensions, auto_next);
        let class         = "integer".to_string();
        let name          = "hier".to_string();
        let title         = "hier title".to_string();
        let printer       = stdout_printer();

        let configuration =
            HierConfig { descriptor, generator, class, name, title, printer };

        Hier::new(configuration)
    }

    fn compute_events_per_entry(hier_integer: &Hier, level: usize) -> i64 {
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

    fn compute_len(hier_integer: &Hier, level: usize, set: HierSet, events: i64) -> usize {
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

    fn check_sizes(hier_integer: &Hier, events: i64, verbose: bool) {
        for level in 0..hier_integer.stats.len() {
            let expected_all_len  = compute_len(hier_integer, level, HierSet::All,  events);
            let expected_live_len = compute_len(hier_integer, level, HierSet::Live, events);

            let actual_all_len    = hier_integer.stats[level].all_len();
            let actual_live_len   = hier_integer.stats[level].live_len();

            if verbose {
                println!("check_sizes:  at level {}, events {}", level, events);

                println!("    live {} == {}",
                    actual_live_len, expected_live_len);

                println!("    all {} == {}",
                    actual_all_len, expected_all_len);
            }

            assert!(actual_all_len  == expected_all_len );
            assert!(actual_live_len == expected_live_len);
        }
    }

    // This is a fairly straightforward test that just pushes a lot of
    // values into a Hier struct.  It is long because it takes a fair
    // number of operations to force higer-level statistics into existence.

    fn simple_hier_test() {
        let     auto_next      = 4;
        let     level_0_period = 4;
        let     signed_auto    = auto_next as i64;
        let mut events         = 0;
        let mut sum_of_events  = 0;
        let     generator      = RunningGenerator::new();
        let     generator      = Rc::from(RefCell::new(generator));
        let mut hier_integer   = make_hier(generator, level_0_period, auto_next);

        // Check that the struct matches our expectations.

        assert!(auto_next as i64 == hier_integer.auto_next);
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
        println!("simple_hier_test:  print 1 at {}", events);
        hier_integer.print();
        println!("simple_hier_test:  print 1.5 at {}", events);

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
        println!("simple_hier_test:  print 2 at {}", events);
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

        println!("simple_hier_test:  print 3 at {}", events);
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

        let stat_rc      = hier_integer.stats[2].newest().unwrap();
        let stat_borrow  = stat_rc.borrow();
        let stat         = stat_borrow.to_rustics();

        println!("simple_hier_test:  print 4 at {}", events);
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
        fn visit(&mut self, _member: &mut dyn Rustics) {
            self.count += 1;
        }
    }

    // Shove enough events into the stat to get a level 3 entry.  Check that
    // the count and the mean of the level 3 stat match our expectations.

    fn long_test() {
        let     auto_next      = 2;
        let     level_0_period = 4;
        let     generator      = RunningGenerator::new();
        let     generator      = Rc::from(RefCell::new(generator));
        let mut hier_integer   = make_hier(generator, level_0_period, auto_next);
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

        {
            let stat_rc        = hier_integer.stats[3].newest().unwrap();
            let stat_borrow    = stat_rc.borrow();
            let stat           = stat_borrow.to_rustics();

            let events_in_stat = (events - 1) as f64;
            let sum            = (events_in_stat * (events_in_stat + 1.0)) / 2.0;
            let mean           = sum / events_in_stat;

            println!("long_test:  stats.mean() {}, expected {}", stat.mean(), mean);

            assert!(stat.count() as i64 == events - 1               );
            assert!(stat.count() as i64 == events_per_level_3 as i64);
            assert!(stat.mean()         == mean                     );

            hier_integer.print_all(None, None);
        }

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

    fn sample_usage() {
    }

    #[test]
    fn run_tests() {
        println!("Running the hierarchical stats tests.");
        simple_hier_test();
        sample_usage();
        long_test();
    }
}
