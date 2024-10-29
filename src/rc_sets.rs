//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!
//! ## Type
//!
//! * RcSet
//!     * RcSet implements a set that can contain Rustics instances
//!       and other RcSet instances.
//!
//!     * Members of an RcSet are kept as Rc instances to allow for
//!       single-threaded sharing.
//!
//! ## Example
//!```
//!    // RcSet and ArcSet provide a nearly identical interface.  This
//!    // code is mostly lifted from the ArcSet comments.
//!
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use std::time::Instant;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::rc_sets::RcSet;
//!
//!    // Create a set.  We're expecting 8 statistics instances but no
//!    // subsets, so we set those hints appropriately.  The  default
//!    // print output goes to stdout, and that's fine for an example, so
//!    // just give "None" to accept the default.
//!
//!    let mut set = RcSet::new("Main Statistics", 8, 0, None);
//!
//!    // Add an instance to record query latencies.  It's a time
//!    // statistic, so we need a timer.  Use an adapter for the Rust
//!    // standard Duration timer.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    // The add_running_timer method is a helper function for creating
//!    // RunningTime instances.
//!
//!    let mut query_latency =
//!         set.add_running_time("Query Latency", timer);
//!
//!    // By way of example, we assume that the queries are single-
//!    // threaded, so we can use the record_event() method to query
//!    // the timer and restart it.
//!    //
//!    // So record one event time for the single-threaded case.  The
//!    // record_event code uses the timer we passed at construction.
//!
//!    query_latency.borrow_mut().record_event();
//!
//!    // For the multithreaded case, you can use DurationTimer manually.
//!    // Usually, ArcSet instances are more convenient for multithreaded
//!    // applications.
//!
//!    let mut local_timer = DurationTimer::new();
//!
//!    // Do our query.
//!    // ...
//!    // Apply a lock to get to query_latency...
//!
//!    let mut lock = query_latency.borrow_mut();
//!
//!    lock.record_time(local_timer.finish() as i64);
//!
//!    drop(lock);
//!
//!    // If you want to use your own timer, you'll need to implement the
//!    // Timer trait to initialize the RunningTime instance, but you can
//!    // use it directly to get data. Let's use DurationTimer directly
//!    // as an example.  Make a new instance for this example.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    let mut query_latency =
//!        set.add_running_time("Custom Timer", timer.clone());
//!
//!    // Start the Duration timer.
//!
//!    let start = Instant::now();
//!
//!    // Do our query.
//!
//!    // Now get the elapsed time.  DurationTimer works in nanoseconds,
//!    // so use as_nanos() to get the tick count.
//!
//!    assert!(timer.borrow().hz() == 1_000_000_000);
//!    let time_spent = start.elapsed().as_nanos();
//!
//!    query_latency.borrow_mut().record_time(time_spent as i64);
//!
//!    // Print our statistics.  This example has only one event recorded.
//!
//!    let query_borrow = query_latency.borrow();
//!
//!    query_borrow.print();
//!
//!    assert!(query_borrow.count() == 1);
//!    assert!(query_borrow.mean() == time_spent as f64);
//!    assert!(query_borrow.standard_deviation() == 0.0);
//! 
//!```

use std::rc::Rc;
use std::cell::RefCell;
use super::Rustics;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOpts;
use super::Units;
use super::TimerBox;
use super::stdout_printer;
use super::counter::Counter;
use super::make_title;

use super::running_integer::RunningInteger;
use super::running_time::RunningTime;
use super::integer_window::IntegerWindow;
use super::time_window::TimeWindow;

pub type RusticsRc = Rc<RefCell<dyn Rustics>>;
pub type RcSetBox  = Rc<RefCell<RcSet>>;

/// The RcTraverser trait defines an interface the user can implement
/// to traverse the elements in an Rc set hierarchy.

pub trait RcTraverser {
    /// This method is invoked on each set in the set, including
    /// the top-level set.

    fn visit_set(&mut self, set: &mut RcSet);

    /// This method is invoked on each statistics instance in the
    /// set.

    fn visit_member(&mut self, member: &mut dyn Rustics);
}

/// RcSet is the base implementation type of the set.

#[derive(Clone)]
pub struct RcSet {
    name:       String,
    title:      String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsRc>,
    subsets:    Vec<RcSetBox>,
    printer:    PrinterBox,
}

impl RcSet {

    /// Create a new set.
    ///
    /// The "members_hint" and "subsets_hint" parameters are hints as to the number
    /// of elements to be expected.  "members_hint" refers to the number of Rustics
    /// statistics in the set.  These hints can improve performance a bit.

    pub fn new(name_in: &str, members: usize, subsets: usize, printer: PrinterOption) -> RcSet {
        let name    = String::from(name_in);
        let title   = String::from(name_in);
        let id      = usize::MAX;
        let next_id = 1;
        let members = Vec::with_capacity(members);
        let subsets = Vec::with_capacity(subsets);

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        RcSet { name, title, id, next_id, members, subsets, printer }
    }

    /// Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Traverses the Rustics instances and subsets, invoking a user-defined
    /// function for each member of the set.

    pub fn traverse(&mut self, traverser: &mut dyn RcTraverser) {
        traverser.visit_set(self);

        for member in self.members.iter() {
            let member = &mut *((**member).borrow_mut());

            traverser.visit_member(member);
        }

        for subset in self.subsets.iter() {
            let mut subset = (**subset).borrow_mut();

            subset.traverse(traverser);
        }
    }

    /// Prints the set and all its constituents (subsets and Rustics instances).

    pub fn print(&self) {
        self.print_opts(None, None);
    }

    pub fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        for member in self.members.iter() {
            let member  = (**member).borrow();
            let printer = printer.clone();

            if let Some(title) = title {
                let title = make_title(title, &member.name());
                let title = Some(title.as_str());

                member.print_opts(printer, title);
            } else {
                member.print_opts(printer, None);
            }
        }

        for subset in self.subsets.iter() {
            let subset  = (**subset).borrow();
            let printer = printer.clone();

            if let Some(title) = title {
                let title = make_title(title, &subset.name());
                let title = Some(title.as_str());

                subset.print_opts(printer, title);
            } else {
                subset.print_opts(printer, None);
            }
        }
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = String::from(title);

        for subset in self.subsets.iter() {
            let mut subset = (**subset).borrow_mut();
            let     title  = make_title(title, &subset.name);

            subset.set_title(&title);
        }

        for member in self.members.iter() {
            let mut member = (**member).borrow_mut();
            let     title  = make_title(title, &member.name());

            member.set_title(&title);
        }
    }

    /// Does a recursive clear of all Rustics instances in the set and its
    /// entire subset hierarachy.

    pub fn clear(&mut self) {
        for subset in self.subsets.iter() {
            let mut subset = (**subset).borrow_mut();
            subset.clear();
        }

        for member in self.members.iter() {
            let mut member = (**member).borrow_mut();
            member.clear();
        }
    }

    /// Adds a the Rustics instance to the set.

    pub fn add_member(&mut self, member: RusticsRc) {
        let mut stat   = member.borrow_mut();
        let     title  = make_title(&self.title, &stat.name());

        stat.set_title(&title);
        stat.set_id(self.next_id);
        self.next_id += 1;
        drop(stat);

        self.members.push(member);
    }

    /// Creates a RunningInteger instance and adds it to the set.

    pub fn add_running_integer(&mut self, name: &str, units: Option<Units>) -> RusticsRc {
        let     printer = Some(self.printer.clone());
        let mut member  = RunningInteger::new(name, printer);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = Rc::from(RefCell::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a IntegerWindow statistics instance and adds it to the set.

    pub fn add_integer_window(&mut self, name: &str, window_size: usize, units: Option<Units>)
            -> RusticsRc {
        let     printer = Some(self.printer.clone());
        let mut member  = IntegerWindow::new(name, window_size, printer);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member  = Rc::from(RefCell::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningTime instance and adds it to the set.

    pub fn add_running_time(&mut self, name: &str, timer: TimerBox) -> RusticsRc {
        let printer = Some(self.printer.clone());
        let member  = RunningTime::new(name, timer, printer);
        let member  = Rc::from(RefCell::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a TimeWindow instance and adds it to the set.

    pub fn add_time_window(&mut self, name: &str, window_size: usize, timer: TimerBox) -> RusticsRc {
        let printer = Some(self.printer.clone());
        let member  = TimeWindow::new(name, window_size, timer, printer);
        let member  = Rc::from(RefCell::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a Counter instance and adds it to the set.

    pub fn add_counter(&mut self, name: &str, units: Option<Units>) -> RusticsRc {
        let printer    = Some(self.printer.clone());
        let title      = None;

        let print_opts = Some(PrintOpts { printer, title, units });
        let member     = Counter::new(name, print_opts);
        let member     = Rc::from(RefCell::new(member));

        self.add_member(member.clone());
        member
    }

    /// Removes a Rustics element from the set.

    pub fn remove_stat(&mut self, target: RusticsRc) -> bool {
        let mut found     = false;
        let mut i         = 0;
        let     member    = (*target).borrow_mut();
        let     target_id = member.id();

        drop(member);

        for rc in self.members.iter() {
            let member = (**rc).borrow_mut();
            found = member.id() == target_id;

            if found {
                break;
            }

            i += 1;
            drop(member);
        }

        if found {
            self.members.remove(i);
        }

        found
    }

    /// Creates a new subset and adds it to the set.

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> RcSetBox {
        let     printer = Some(self.printer.clone());
        let mut subset  = RcSet::new(name, members, subsets, printer);
        let     title   = make_title(&self.title, name);

        subset.set_title(&title);
        subset.set_id(self.next_id);
        self.next_id += 1;

        let subset = Rc::from(RefCell::new(subset));

        self.subsets.push(subset.clone());
        subset
    }

    /// Removes a subset from the set.

    pub fn remove_subset(&mut self, target: &RcSetBox) -> bool {
        let mut found     = false;
        let mut i         = 0;
        let     subset    = (**target).borrow_mut();
        let     target_id = subset.id();

        drop(subset);

        for subset in self.subsets.iter() {
            let subset = (**subset).borrow_mut();
            found = subset.id() == target_id;

            if found {
                break;
            }

            i += 1;
        }

        if found {
            self.subsets.remove(i);
        }

        found
    }

    // The following methods are for internal use only.

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::continuing_box;
    use crate::hier::Hier;

    struct TestTraverser {
        pub members:  i64,
        pub sets:     i64,
    }

    impl TestTraverser {
        pub fn new() -> TestTraverser {
            println!(" *** making an rc traverser");
            TestTraverser { members:  0, sets:  0 }
        }
    }

    impl RcTraverser for TestTraverser {
        fn visit_member(&mut self, member: &mut dyn Rustics) {
            println!(" *** visiting rc member  \"{}\"", member.name());
            self.members += 1;
        }

        fn visit_set(&mut self, set: &mut RcSet) {
            println!(" *** visiting rc set     \"{}\"", set.name());
            self.sets += 1;
        }
    }

    fn add_stats(parent: &mut RcSet) {
        let lower = -64;
        let upper =  64;

        let parent_set = parent;

        for _i in 0..4 {
            let     subset  = parent_set.add_subset("generated subset", 4, 4);
            let mut subset  = (*subset).borrow_mut();

            let window      = subset.add_integer_window("generated subset window", 32, None);
            let running     = subset.add_running_integer("generated subset running", None);

            let mut window  = (*window).borrow_mut();
            let mut running = (*running).borrow_mut();

            for i in lower..upper {
                window.record_i64(i);
                running.record_i64(i);
            }
        }

        let     counter = parent_set.add_counter("generated counter", None);
        let mut counter = (*counter).borrow_mut();

        for i in 0..upper {
            counter.record_i64(i);
        }
    }

    pub fn simple_test() {
        let lower    = -32;
        let upper    = 32;

        // Create the parent set for all the Rustics instances.

        let mut set = RcSet::new("parent set", 4, 4, None);

        // Add integer statistics instances, both a running total and a window.

        let window  = set.add_integer_window("window", 32, None);
        let running = set.add_running_integer("running", None);

        let window_timer:  TimerBox = continuing_box();
        let running_timer: TimerBox = continuing_box();

        let time_window  = set.add_time_window("time window", 32, window_timer);
        let running_time = set.add_running_time("running time", running_timer);

        // Now test recording data.

        let mut window_stat       = (*window).borrow_mut();
        let mut running_stat      = (*running).borrow_mut();

        let mut time_window_stat  = (*time_window).borrow_mut();
        let mut running_time_stat = (*running_time).borrow_mut();

        for i in lower..upper {
            window_stat.record_i64(i);
            running_stat.record_i64(i);

            time_window_stat.record_event();
            running_time_stat.record_event();
        }

        // Check that the titles are being set correctly.

        let set_title = set.title();
        assert!(set_title == "parent set");

        assert!(running_time_stat.title() == make_title(&"parent set", &"running time"));
        assert!(time_window_stat.title()  == make_title(&"parent set", &"time window" ));
        assert!(running_stat.title()      == make_title(&"parent set", &"running"     ));
        assert!(window_stat.title()       == make_title(&"parent set", &"window"      ));

        //  Test subset titles.

        let     subset       = set.add_subset("subset", 0, 0);
        let mut subset       = (*subset).borrow_mut();
        let     subset_title = subset.title();

        let     subset_stat  = subset.add_running_integer("subset stat", None);
        let     subset_stat  = (*subset_stat).borrow_mut();

        assert!(subset_title        == make_title(&set_title, "subset"         ));
        assert!(subset_stat.title() == make_title(&subset_title, &"subset stat"));

        //  Drop the locks so that we can print the set.

        drop(window_stat);
        drop(running_stat);
        drop(subset_stat);
        drop(subset);
        drop(time_window_stat);
        drop(running_time_stat);

        set.print();

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** rc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 5);
        assert!(traverser.sets == 2);

        // Add more subsets to test removal operations.

        let subset_1 = set.add_subset("subset 1", 4, 4);
        let subset_2 = set.add_subset("subset 2", 4, 4);

        add_stats(&mut (*subset_1).borrow_mut());
        add_stats(&mut (*subset_2).borrow_mut());

        println!("=========== Hierarchical Print");
        set.print();

        // Remove a subset and check that it goes away.

        let found = set.remove_subset(&subset_1);
        assert!(found);

        let found = set.remove_subset(&subset_1);
        assert!(!found);

        // Remove two stats and check that they go away.
        //
        // First, do the remove operations.  We must clone the
        // rc instances since the call moves them.

        let found = set.remove_stat(window.clone());
        assert!(found);

        let found = set.remove_stat(running.clone());
        assert!(found);

        // Now check that the stats went away

        let found = set.remove_stat(window);
        assert!(!found);

        let found = set.remove_stat(running);
        assert!(!found);
    }

    fn new_hier_integer() -> Hier {
        crate::hier::tests::make_hier(4, 8)
    }

    fn sample_usage() {
        // The last two parameters to new() and add_subset are size hints.
        // They are only hints.
        //
        //  Create the parent set and add a subset.

        let mut set     = RcSet::new("sample usage parent", 0, 0, None);
        let     subset  = set.add_subset("subset", 0, 0);
        let mut subset  = (*subset).borrow_mut();

        // Create a running integer instance.

        let     running = subset.add_running_integer("generated subset running", None);
        let mut running = (*running).borrow_mut();

        // Now try a timer window.

        let     window_timer    = continuing_box();
        let     time_window     = set.add_time_window("time window", 32, window_timer);
        let mut time_window     = (*time_window).borrow_mut();
        let mut timer: TimerBox = continuing_box();

        // Do a quick sanity test.

        assert!(time_window.class() == "time");

        (*timer).borrow_mut().start();

        //  Record some data.

        for i in -32..64 {
            running.record_i64(i);
            time_window.record_event();
            time_window.record_interval(&mut timer);
        }

        // Drop the locks before trying to print.

        drop(running);
        drop(subset);
        drop(time_window);

        // Do a minimal test of "add".

        let member = RunningInteger::new("added as member", None);
        let member = Rc::from(RefCell::new(member));
        set.add_member(member);

        set.print();

        let hier_integer = new_hier_integer();
        let member       = Rc::from(RefCell::new(hier_integer));

        set.add_member(member);

        set.print();
    }

    #[test]
    pub fn run_tests() {
        simple_test();
        sample_usage();
    }
}
