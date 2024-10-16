//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use super::Rustics;
use super::Histogram;
use super::LogHistogram;
use super::Printer;
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

// This struct is used to describe one level of the statistics
// hierarchy.  "period" specifies the number of pushes into this
// window before a sum statistic is pushed to the upper level.
//
// "retention" specifies the total number of statistics to keep
// around for queries.  It must be at least "period" elements, but
// can be more to keep more history.

#[derive(Clone, Copy)]
pub struct HierDimension {
    period:        usize,   // the number of statistics to be summed for the next level
    retention:     usize,   // the number of statistics to retain for queries.
}

impl HierDimension {
    pub fn new(period: usize, retention: usize) -> HierDimension {
        if retention < period {
            panic!("HierDimension::new:  The retention count must be at the period length.");
        }

        HierDimension { period, retention }
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn retention(&self) -> usize {
        self.retention
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

// The exporter needs to be downcasted to be used, so
// provide that interface.

pub trait HierExporter {
    fn as_any    (&self)     -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait HierTraverser {
    fn visit(&mut self, member: &mut dyn Rustics);
}

//
// HierGenerator is implemented for a type implementing
// Rustics.  This interfaces with the struct Hier coder
// to provide hierarchical statistics.  This trait
// provides functions needed by the Hier code that are
// not bound to a specific instance of a Rustics object,
// and so can't be in trait Rustics.  It is an abstraction
// and extension of the impl code of the type that
// implements Rustics.  For example, there is a
// HierGenerator implementation for the RunningInteger
// type.
//

pub trait HierGenerator {
    fn make_from_exporter(&self, name: &str, printer: PrinterBox, exports: ExporterRc) -> MemberRc;

    fn make_member       (&self, name: &str, printer: PrinterBox)  -> MemberRc;
    fn make_exporter     (&self)                                   -> ExporterRc;

    fn push              (&self, exports: ExporterRc, member: MemberRc);
}

//
//  The HierMember trait is used to extend a specific object
//  implementing the Rustics trait to work with the Hier
//  type.  The code for the Hier type and the HierGenerator
//  just need to be able to upcast and downcast into the member types.
//  It is thus a very rough abstraction of a Rustics struct.
//  The as_any* function actually are available via the Rustics
//  trait, but it's easier to use them without that step.
//

pub trait HierMember {
    fn to_rustics    (&self    ) -> &dyn Rustics;
    fn to_rustics_mut(&mut self) -> &mut dyn Rustics;
    fn to_histogram  (&    self) -> &dyn Histogram;
    fn as_any        (&self    ) -> &dyn Any;
    fn as_any_mut    (&mut self) -> &mut dyn Any;
}

//
// This structure implements an implementation of hierarchical
// statistics using a HierGenerator object and HierMember
// structs.
//

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
        let     id            = usize::MAX;
        let     advance_count = 0;
        let     event_count   = 0;
        let mut stats         = Vec::with_capacity(dimensions.len());

        //
        // Create the set of windows that we use to hold all
        // the actual statistics objects.
        //

        for dimension in &dimensions {
            stats.push(Window::new(dimension.retention, dimension.period));
        }

        //
        // Make the first statistics so that we are ready to record data.
        //

        let member = generator.borrow_mut().make_member(&name, printer.clone());

        stats[0].push(member);

        Hier {
            dimensions,   generator,  stats,
            name,         title,      id,
            class,        auto_next,  advance_count,
            event_count,  printer
        }
    }

    // Returns the newest statistic at the lowest level, which
    // is the only statistic that records data.  The other
    // members are all read-only.

    pub fn current(&self) -> MemberRc {
        let member = self.stats[0].newest().unwrap();

        member.clone()
    }

    // Print the given element in the statistics matrix.

    pub fn print_index_opts(&self, index: HierIndex, printer: PrinterOption, title: Option<&str>) {
        self.local_print(index, printer, title);
    }

    // Print all members in this object.

    pub fn print_all(&self, printer: PrinterOption, title: Option<&str>) {
        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.all_len() {
                let index = HierIndex::new(HierSet::All, i, j);

                self.local_print(index, printer.clone(), title)
            }
        }
    }

    // Clear all the statistics from the windows.  Reset the
    // event and advance counters. Finally, push a new level 0
    // statistic to receive data.
    //
    // This operation sets the struct back to its initial state.

    pub fn clear_all(&mut self) {
        self.advance_count = 0;
        self.event_count   = 0;

        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.all_len() {
                let     target = self.stats[i].index_all(j);
                let mut target = target.unwrap().borrow_mut();

                target.to_rustics_mut().clear();
            }
        }
    }

    // Provide a traverser for users to look at the that are live.

    pub fn traverse_live(&mut self, traverser: &mut dyn HierTraverser) {
        for level in &mut self.stats {
            for member in level.iter_live() {
                let mut borrow  = member.borrow_mut();
                let     rustics = borrow.to_rustics_mut();

                traverser.visit(rustics);
            }
        }
    }

    // Push a new level 0 statistics into the level 0 window.
    // Update the upper levels as needed.  The user can
    // call this directly, use auto_advance, or do both.

    pub fn advance(&mut self) {
        // Increment the advance op count.  This counts the
        // number of statistics objects pushed, and thus tells
        // us when we need to push a new higher-level object.

        self.advance_count += 1;

        // Now move up the stack.

        let mut advance_point = 1;
        let     generator     = self.generator.borrow();

        // Check whether we have enough new objects at a given level
        // to push a new sum of those statistics to a higher level.

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

        // Create the new statistic to collect data and push it into the
        // level zero window.

        let member = generator.make_member(&self.name, self.printer.clone());

        self.stats[0].push(member);
    }

    pub fn live_len(&self, level: usize) -> usize {
        self.stats[level].live_len()
    }

    pub fn all_len(&self, level: usize) -> usize {
        self.stats[level].all_len()
    }

    // Return the total number of statistics events seen by
    // all the statistics seen by this window since its
    // create or the last clear_all invocation, whichever
    // is later.

    pub fn event_count(&self) -> i64 {
        self.event_count
    }

    // Print one statistics object using the Rustics trait.
    // We always append the indices to the title that is
    // given.

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
                printer
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
                printer.print(&format!("  That index ({}) is out of bounds.", which));
                return;
            };

        // Downcast to the Rustics level and print.

        let target = target.borrow();
        target.to_rustics().print_opts(printer_opt, title_opt);
    }

    // Create an exporter for when we need to sum a group of
    // statistics.  The exporter accumulates the sums of all
    // the data necessary for the actual Rustics implementation.

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

    // Check the event count to see whether it's time to
    // push a new level 0 statistics.  This routine
    // implements the auto_next feature.

    fn check_and_advance(&mut self) {
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
        
        // Advance the event count.  This routine should only be
        // called when a statistical value is being recorded.

        self.event_count += 1;
    }
}

// Implement the Rustics trait for the Hier object.  It
// returns data mostly from the newest level 0 object,
// which is the only one receiving data.

impl Rustics for Hier {
    fn record_i64(&mut self, value: i64) {
        self.check_and_advance();

        let     member  = self.stats[0].newest_mut().unwrap();
        let mut borrow  = member.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_i64(value)
    }

    fn record_f64(&mut self, sample: f64) {
        self.check_and_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_f64(sample);
    }

    fn record_event(&mut self) {
        self.record_i64(1);
    }

    fn record_time(&mut self, sample: i64) {
        self.check_and_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_time(sample);
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        self.check_and_advance();

        let     current = self.current();
        let mut borrow  = current.borrow_mut();
        let     rustics = borrow.to_rustics_mut();

        rustics.record_interval(timer);
    }

    // We return the name and title of the Hier structure itself.
    // We do the same for class, as well.

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
        let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

        self.local_print(index, printer, title);
    }

    // The title is kept in the Hier object.

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<Hier>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn histogram(&self) -> LogHistogram {
        let current = self.current();
        let borrow  = current.borrow();
        let rustics = borrow.to_rustics();

        rustics.histogram()
    }
}

impl Histogram for Hier {
    fn log_histogram(&self) -> LogHistogram {
        let current   = self.current();
        let borrow    = current.borrow();
        let log_histo = borrow.to_histogram();

        log_histo.log_histogram()
    }

    fn print_histogram(&self, printer: &mut dyn Printer) {
        let current   = self.current();
        let borrow    = current.borrow();
        let log_histo = borrow.to_histogram();

        log_histo.print_histogram(printer);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::run_histogram_tests;
    use crate::stdout_printer;
    use crate::integer_hier::IntegerHier;
    use crate::integer_hier::IntegerHierConfig;
    use crate::running_integer::RunningInteger;
    use crate::time_hier::TimeHier;
    use crate::time_hier::TimeHierConfig;
    use crate::running_time::RunningTime;

    // Make a Hier struct for testing.  The tests use the RunningInteger
    // implementation via IntegerHier.
    //
    // Make a 4-level hierarchical statistic for testing.

    pub fn make_hier(level_0_period: usize, auto_next: usize) -> Hier {
        let     levels      = 4;
        let     dimension   = HierDimension::new(level_0_period, 3 * level_0_period);
        let mut dimensions  = Vec::<HierDimension>::with_capacity(levels);

        // Push the level 0 descriptor.

        dimensions.push(dimension);

        // Create a hierarchy.  Use a period of 4 level at level 1,
        // and just add 2 as we go up.  This will keep all the
        // periods distinct.  We force the total window size to
        // 3 times the period, which is fine for testing.  That
        // might become too costly in actual usage.

        let mut period = 4;

        for _i in 1..levels {
            let dimension = HierDimension::new(period, 3 * period);

            dimensions.push(dimension);

            period += 2;
        }

        // Finish creating the Hier description structure.  This
        // just describes the windows and how the statistics
        // structures are advanced.

        let auto_next  = Some(auto_next as i64);
        let descriptor = HierDescriptor::new(dimensions, auto_next);

        // Now create the RunningInteger-based Hier struct via
        // IntegerHier, which does some of the work for
        // us.

        let name    = "hier".to_string();
        let title   = "hier title".to_string();
        let printer = stdout_printer();

        // Finally, create the configuration description for the
        // constructor.

        let configuration =
            IntegerHierConfig { descriptor, name, title, printer };

        // Make the actual Hier structure.  new_hier() handles the
        // parameters specific for using RunningInteger statistics.

        IntegerHier::new_hier(configuration)
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
        let mut hier_integer   = make_hier(level_0_period, auto_next);
        let     hier_integer_2 = make_hier(level_0_period, auto_next);

        // Do a quick sanity test on equals().

        assert!( hier_integer.equals(&hier_integer));
        assert!(!hier_integer.equals(&hier_integer_2));

        // Check that the struct matches our expectations.

        let live_len  = hier_integer.stats[0].live_len();
        let all_len   = hier_integer.stats[0].all_len();
        let period    = hier_integer.dimensions[0].period;

        assert!(signed_auto == hier_integer.auto_next);
        assert!(all_len     == 1                     );
        assert!(live_len    == 1                     );
        assert!(period      == level_0_period        );

        let expected_count = auto_next as u64;

        // Push one full window and see whether the data
        // and structure matches what we expect as we
        // record each event.

        for i in 0..signed_auto {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            if i < signed_auto - 1 {
                let mean  = sum_of_events as f64 / (i + 1) as f64;

                let live_len_0 = hier_integer.stats[0].live_len();
                let all_len_0  = hier_integer.stats[0].all_len();
                let count      = (i + 1) as u64;

                assert!(all_len_0              == 1    );
                assert!(live_len_0             == 1    );
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


        // Record two windows worth of events and check that
        // our size expectations are correct.  Also, keep the
        // sum of the last "window" of events so that we can
        // check the mean.  Only the lawt "window" events should
        // be used to compute the mean.

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

        // Check that we have at least one level 1 object.  This
        // is a test of the test itself, for the most part.

        let expected_len = compute_len(&hier_integer, 1, HierSet::All,  events);
        let actual_len   = hier_integer.stats[1].all_len();

        assert!(expected_len > 0);
        assert!(actual_len == expected_len);

        // Now force a level 2 statistics.

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

        // Do a sanity test on equals.

        assert!(!hier_integer.equals(stat));

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
        let mut hier_integer   = make_hier(level_0_period, auto_next);
        let mut events         = 0;

        // Record data until there's a level 3 sum statistic.

        while hier_integer.stats[3].all_len()  == 0 {
            events += 1;
            hier_integer.record_i64(events);

            check_sizes(&hier_integer, events, false);
        }

        // Now see that all the sizes, etc, match what we
        // expect.

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

        // Do a quick test of the traverser.  It should see each
        // statistic in the matrix.

        let mut traverser = TestTraverser::new();
        let mut predicted = 0;

        hier_integer.traverse_live(&mut traverser);

        for level in 0..hier_integer.dimensions.len() {
            predicted += hier_integer.live_len(level) as i64;
        }

        println!("long_test:  traversed {} stats structs, predicted {}",
            traverser.count, predicted);
        assert!(traverser.count == predicted);

        // Do a sanity test on the members.

        let member_opt    = hier_integer.stats[1].newest().unwrap();
        let member_borrow = member_opt.borrow();
        let member        = member_borrow.as_any().downcast_ref::<RunningInteger>();

        let proper_type =
            match member {
                Some(_) => { true  }
                None    => { false }
            };

        let rustics = member.unwrap().to_rustics();

        println!("long_test:  got \"{}\" for class", rustics.class());

        assert!(proper_type);
        assert!(rustics.class() == "integer");
    }

    fn test_time_hier_sanity() {
        //  First, just make a generator and a member, then record one event.

        let     name         = "time_hier sanity test".to_string();
        let     title        = "time_hier sanity test title".to_string();
        let     timer        = crate::tests::ContinuingTimer::new(1_000_0000);
        let     timer        = Rc::from(RefCell::new(timer));
        let     printer      = stdout_printer();

        // Create the dimensions.

        let dimension_0 = HierDimension::new(4, 4);
        let dimension_1 = HierDimension::new(100, 200);

        let dimensions = vec![ dimension_0.clone(), dimension_1.clone() ];

        let auto_next    = 20;
        let auto_advance = Some(auto_next);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);

        // Now make an actual time_hier struct from a configuration.

        let configuration =
            TimeHierConfig { descriptor, timer, name, title, printer };

        let mut hier = TimeHier::new_hier(configuration);

        // Now force a level 1 statistics.

        let mut events             = 0;
        let     events_per_level_1 = auto_next * dimension_0.period() as i64;

        for i in 0..2 * events_per_level_1 {
            hier.record_time(i + 1);

            events += 1;
        }

        assert!(hier.event_count() == events);
        hier.print();

        // Do a sanity test on the members.

        let member_opt    = hier.stats[1].newest().unwrap();
        let member_borrow = member_opt.borrow();
        let member        = member_borrow.as_any().downcast_ref::<RunningTime>();

        let proper_type =
            match member {
                Some(_) => { true  }
                None    => { false }
            };

        let rustics = member.unwrap().to_rustics();

        assert!(proper_type);
        assert!(rustics.count() == events_per_level_1 as u64);
        assert!(rustics.class() == "time");
        println!("test_time_hier_sanity:  got \"{}\" for class", rustics.class());
    }

    fn sample_usage() {
        // Make a descriptor of the first level.  We have chosen to sum 1000
        // level 0 RunningInteger structs into one level 1 RunningInteger
        // struct.  This level is large, so we will keep only 1000 level 0
        // structs in the window.

        let dimension_0 = HierDimension::new(1000, 1000);

        // At level 1, we want to sum 100 level 1 statistics into one level 2
        // statistics.  This level is smaller, so let's retain 200
        // RunningInteger structs here.

        let dimension_1 = HierDimension::new(100, 200);

        // Level two isn't summed, so the period isn't used.  Tell it to
        // sum one event to keep the contructor happy.  Let's pretend this
        // level isn't used much, so retain only 100 structs in it.

        let dimension_2 = HierDimension::new(1, 100);

        //  Now create the Vec.  Save the dimension structs for future use.

        let dimensions =
            vec![
                dimension_0.clone(), dimension_1.clone(), dimension_2.clone()
            ];

        // Now create the entire descriptor for the hier struct.  Let's
        // record 2000 events into each level 0 RunningInteger instance.

        let auto_advance = Some(2000);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);

        // Now create some items used by Hier to do printing.

        let name    = "test hierarchical integer".to_string();
        let title   = "test hierarchical integer".to_string();
        let printer = stdout_printer();

        // Finally, create the configuration description for the
        // constructor.

        let configuration =
            IntegerHierConfig { descriptor, name, title, printer };

        // Now make the Hier instance.

        let mut integer_hier = IntegerHier::new_hier(configuration);

        // Now record some events with boring data.

        let mut events   = 0;
        let auto_advance = auto_advance.unwrap();

        for i in  0..auto_advance {
            events += 1;
            integer_hier.record_i64(i + 10);
        }

        // We have just completed the first level 0 structure, but
        // the implementation creates the next struct only when
        // it has data to record, so there should be only one level
        // zero struct, and nothing at level 1 or level 2.

        assert!(integer_hier.event_count() == events);
        assert!(integer_hier.count()       == events as u64);
        assert!(integer_hier.live_len(0)   == 1     );
        assert!(integer_hier.live_len(1)   == 0     );
        assert!(integer_hier.live_len(2)   == 0     );

        // Now record some data to force the creation of
        // the second level 1 struct.

        events += 1;
        integer_hier.record_i64(10);

        // The new level 0 struct should have only one event
        // recorded.  The Rustics implementatio for Hier returns
        // the data in the current level 0 struct, so check it.

        assert!(integer_hier.event_count() == events);
        assert!(integer_hier.count()       == 1     );
        assert!(integer_hier.live_len(0)   == 2     );
        assert!(integer_hier.live_len(1)   == 0     );
        assert!(integer_hier.live_len(2)   == 0     );

        let events_per_level_1 =
            auto_advance * dimension_0.period() as i64;

        for i in events..events_per_level_1 {
            integer_hier.record_i64(i);
            events += 1;
        }

        // Check the state again.  We need to record one more
        // events to cause the summation at level 0 into level
        // 1.

        let expected_live  = dimension_0.period();
        let expected_count = auto_advance as u64;

        assert!(integer_hier.event_count() == events        );
        assert!(integer_hier.count()       == expected_count);
        assert!(integer_hier.live_len(0)   == expected_live );
        assert!(integer_hier.live_len(1)   == 0             );
        assert!(integer_hier.live_len(2)   == 0             );

        integer_hier.record_i64(42);
        events += 1;

        assert!(integer_hier.live_len(1)   == 1     );
        assert!(integer_hier.event_count() == events);

        // Test the histograms while we have a Hier.

        run_histogram_tests(&mut integer_hier);
    }

    #[test]
    fn run_tests() {
        println!("Running the hierarchical stats tests.");
        simple_hier_test();
        long_test();
        test_time_hier_sanity();
        sample_usage();
    }
}
