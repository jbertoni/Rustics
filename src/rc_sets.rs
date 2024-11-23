//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
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
//!    use rustics::timer;
//!    use rustics::rc_item;
//!    use rustics::rc_item_mut;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::rc_sets::RcSet;
//!
//!    // Create a set.  We're expecting 8 statistics instances but no
//!    // subsets, so we set those hints appropriately.  The  default
//!    // print output goes to stdout, and that's fine for an example, so
//!    // just give "None" to accept the default.
//!
//!    let mut set = RcSet::new("Main Statistics", 8, 0, &None);
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
//!    rc_item_mut!(query_latency).record_event();
//!
//!    // For the multithreaded case, you can use DurationTimer manually.
//!    // Usually, ArcSet instances are more convenient for multithreaded
//!    // applications.
//!
//!    let mut local_timer = DurationTimer::new_box();
//!
//!    // Do our query.
//!    // ...
//!    // Apply a lock to get to query_latency...
//!
//!    let lock = rc_item_mut!(query_latency);
//!
//!    // record_interval() will read the timer for us.
//!
//!    lock.record_interval(&mut local_timer);
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
//!    assert!(timer!(timer).hz() == 1_000_000_000);
//!    let time_spent = start.elapsed().as_nanos();
//!
//!    rc_item_mut!(query_latency).record_time(time_spent as i64);
//!
//!    // Print our statistics.
//!
//!    let query_borrow = rc_item!(query_latency);
//!
//!    query_borrow.print();
//!
//!    assert!(query_borrow.count() == 1);
//!    assert!(query_borrow.mean() == time_spent as f64);
//!    assert!(query_borrow.standard_deviation() == 0.0);
//!```

use std::rc::Rc;
use std::cell::RefCell;
use super::Rustics;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOpts;
use super::PrintOption;
use super::Units;
use super::TimerBox;
use super::counter::Counter;
use super::make_title;
use super::parse_printer;
use super::parse_title;
use super::parse_units;
use super::parse_histo_opts;

use super::running_integer::RunningInteger;
use super::running_time   ::RunningTime;
use super::running_float  ::RunningFloat;

use super::integer_window::IntegerWindow;
use super::time_window   ::TimeWindow;
use super::float_window  ::FloatWindow;

use super::integer_hier::IntegerHier;
use super::integer_hier::IntegerHierConfig;
use super::time_hier   ::TimeHier;
use super::time_hier   ::TimeHierConfig;
use super::float_hier  ::FloatHier;
use super::float_hier  ::FloatHierConfig;

pub type RusticsRc = Rc<RefCell<dyn Rustics>>;
pub type RcSetBox  = Rc<RefCell<RcSet>>;

/// rc_box! is used to create a shareable instance of an RcSet item.

#[macro_export]
macro_rules! rc_box { ($x:expr) => { Rc::from(RefCell::new($x)) } }

/// The rc_item_mut macro converts an RcSet member into a mutable
/// Rustics or subset reference.

#[macro_export]
macro_rules! rc_item_mut { ($x:expr) => { &mut *$x.borrow_mut() } }

/// The rc_item macro converts an RcSet member into a Rustics or subset
/// instance.

#[macro_export]
macro_rules! rc_item { ($x:expr) => { &*$x.borrow() } }

/// The RcTraverser trait defines an interface the user can implement
/// to traverse the members of an Rc set hierarchy.

pub trait RcTraverser {
    /// This method is invoked on each subset in the set and
    /// on the top-level set itself.

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
    print_opts: PrintOption,
}

impl RcSet {

    /// Creates a new set.
    ///
    /// The "members_hint" and "subsets_hint" parameters are hints as to the number
    /// of elements to be expected.  "members_hint" refers to the number of Rustics
    /// statistics in the set.  These hints can improve performance a bit.

    pub fn new(name_in: &str, members: usize, subsets: usize, print_opts: &PrintOption) -> RcSet {
        let name       = String::from(name_in);
        let title      = parse_title(print_opts, &name);
        let id         = usize::MAX;
        let next_id    = 1;
        let members    = Vec::with_capacity(members);
        let subsets    = Vec::with_capacity(subsets);
        let printer    = parse_printer(print_opts);
        let print_opts = print_opts.clone();

        RcSet { name, title, id, next_id, members, subsets, printer, print_opts }
    }

    /// Creates a new RcSet in a box.

    pub fn new_box(name_in: &str, members: usize, subsets: usize, print_opts: &PrintOption)
            -> RcSetBox {
        let rc_set = RcSet::new(name_in, members, subsets, print_opts);

        rc_box!(rc_set)
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
            let member = rc_item_mut!(**member);

            traverser.visit_member(member);
        }

        for subset in self.subsets.iter() {
            let subset = rc_item_mut!(**subset);

            subset.traverse(traverser);
        }
    }

    /// Prints the set and all its constituents (subsets and Rustics instances).

    pub fn print(&self) {
        self.print_opts(None, None);
    }

    /// Prints the set and all its constituents (subsets and Rustics instances)
    /// with the give printer and title.

    pub fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        for member in self.members.iter() {
            let member  = rc_item!(**member);
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
            let subset  = rc_item!(**subset);
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

    /// Returns the current title for the set.

    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// Sets the title for the set.

    pub fn set_title(&mut self, title: &str) {
        self.title = String::from(title);

        for subset in self.subsets.iter() {
            let subset = rc_item_mut!(**subset);
            let title  = make_title(title, &subset.name);

            subset.set_title(&title);
        }

        for member in self.members.iter() {
            let member = rc_item_mut!(**member);
            let title  = make_title(title, &member.name());

            member.set_title(&title);
        }
    }

    /// Does a recursive clear of all Rustics instances in the set and its
    /// entire subset hierarchy.

    pub fn clear(&mut self) {
        for subset in self.subsets.iter() {
            let subset = rc_item_mut!(**subset);

            subset.clear();
        }

        for member in self.members.iter() {
            let member = rc_item_mut!(**member);

            member.clear();
        }
    }

    /// Adds a RusticsRc instance to the set.

    pub fn add_member(&mut self, member: RusticsRc) {
        let work   = member.clone();
        let stat   = rc_item_mut!(work);
        let title  = make_title(&self.title, &stat.name());

        stat.set_title(&title);
        stat.set_id(self.next_id);
        self.next_id += 1;

        self.members.push(member);
    }

    /// Creates a RunningInteger instance and adds it to the set.

    pub fn add_running_integer(&mut self, name: &str, units: Option<Units>) -> RusticsRc {
        let mut member  = RunningInteger::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a IntegerWindow statistics instance and adds it to the set.

    pub fn add_integer_window(&mut self, name: &str, window_size: usize, units: Option<Units>)
            -> RusticsRc {
        let mut member  = IntegerWindow::new(name, window_size, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningInteger instances and adds it to the set.

    pub fn add_integer_hier(&mut self, mut configuration: IntegerHierConfig) -> RusticsRc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = IntegerHier::new_hier(configuration);
        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningTime instance and adds it to the set.

    pub fn add_running_time(&mut self, name: &str, timer: TimerBox) -> RusticsRc {
        let member  = RunningTime::new(name, timer, &self.print_opts);
        let member  = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a TimeWindow instance and adds it to the set.

    pub fn add_time_window(&mut self, name: &str, window_size: usize, timer: TimerBox) -> RusticsRc {
        let member  = TimeWindow::new(name, window_size, timer, &self.print_opts);
        let member  = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningTime instances and adds it to the set.

    pub fn add_time_hier(&mut self, mut configuration: TimeHierConfig) -> RusticsRc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = TimeHier::new_hier(configuration);
        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningFloat instance and adds it to the set.

    pub fn add_running_float(&mut self, name: &str, units: Option<Units>) -> RusticsRc {
        let mut member  = RunningFloat::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a FloatWindow statistics instance and adds it to the set.

    pub fn add_float_window(&mut self, name: &str, window_size: usize, units: Option<Units>)
            -> RusticsRc {
        let mut member  = FloatWindow::new(name, window_size, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningFloat instances and adds it to the set.

    pub fn add_float_hier(&mut self, mut configuration: FloatHierConfig) -> RusticsRc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = FloatHier::new_hier(configuration);
        let member = rc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Counter instance and adds it to the set.

    pub fn add_counter(&mut self, name: &str, units: Option<Units>) -> RusticsRc {
        let member     = Counter::new(name, &self.print_opts);
        let member     = rc_box!(member);

        if let Some(units) = units {
            rc_item_mut!(member).set_units(units);
        }

        self.add_member(member.clone());
        member
    }

    /// Removes a Rustics instance from the set.

    pub fn remove_stat(&mut self, target: RusticsRc) -> bool {
        let mut found     = false;
        let mut i         = 0;
        let     member    = (*target).borrow_mut();
        let     target_id = member.id();

        drop(member);

        for rc in self.members.iter() {
            let member = rc_item_mut!(**rc);

            found = member.id() == target_id;

            if found {
                break;
            }

            i += 1;
        }

        if found {
            self.members.remove(i);
        }

        found
    }

    /// Creates a new subset and adds it to the set.

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> RcSetBox {
        let mut subset  = RcSet::new(name, members, subsets, &self.print_opts);
        let     title   = make_title(&self.title, name);

        subset.set_title(&title);
        subset.set_id(self.next_id);
        self.next_id += 1;

        let subset = rc_box!(subset);

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
            let subset = rc_item_mut!(**subset);

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

    fn make_print_opts(&self, name: &str, print_opts: &PrintOption) -> PrintOption {
        let     printer    = Some(self.printer.clone());
        let     title      = Some(make_title(&self.title, name));
        let     units      = Some(parse_units(print_opts));
        let     histo_opts = Some(parse_histo_opts(print_opts));
        let     print_opts = PrintOpts { printer, title, units, histo_opts };

        Some(print_opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::continuing_box;
    use crate::hier::Hier;
    use crate::timer_mut;
    use crate::arc_sets::tests::make_integer_config;
    use crate::arc_sets::tests::make_time_config;
    use crate::arc_sets::tests::make_float_config;
    use crate::tests::check_printer_box;
    use crate::tests::check_printer_counters;
    use crate::tests::check_printer_count_match;
    use crate::tests::bytes;
    use crate::arc_sets::tests::title_to_print_option;

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
            let subset  = parent_set.add_subset("generated subset", 4, 4);
            let subset  = rc_item_mut!(*subset);

            let window  = subset.add_integer_window("generated subset window", 32, bytes());
            let running = subset.add_running_integer("generated subset running", None);

            let window  = rc_item_mut!(*window);
            let running = rc_item_mut!(*running);

            for i in lower..upper {
                window .record_i64(i);
                running.record_i64(i);
            }
        }

        let counter = parent_set.add_counter("generated counter", bytes());
        let counter = rc_item_mut!(*counter);

        for i in 0..upper {
            counter.record_i64(i);
        }
    }

    pub fn simple_test() {
        let lower    = -32;
        let upper    = 32;

        // Create the parent set for all the Rustics instances.

        let     set = RcSet::new_box("parent set", 4, 4, &None);
        let mut set = set.borrow_mut();

        // Add integer statistics instances, both a running total and a window.

        let window_size = 32;

        let window  = set.add_integer_window("window", window_size, None);
        let running = set.add_running_integer("running", None);

        let window_timer:  TimerBox = continuing_box();
        let running_timer: TimerBox = continuing_box();

        let time_window   = set.add_time_window ("time window",    window_size, window_timer);
        let running_time  = set.add_running_time ("running time",  running_timer);

        let float_window  = set.add_float_window ("float window",  window_size, bytes());
        let running_float = set.add_running_float("running float", bytes());

        // Now test recording data.

        let mut window_stat        = (*window) .borrow_mut();
        let mut running_stat       = (*running).borrow_mut();

        let mut time_window_stat   = (*time_window) .borrow_mut();
        let mut running_time_stat  = (*running_time).borrow_mut();

        let mut float_window_stat  = (*float_window) .borrow_mut();
        let mut running_float_stat = (*running_float).borrow_mut();

        for i in lower..upper {
            let f = i as f64;

            window_stat       .record_i64(i);
            running_stat      .record_i64(i);
            float_window_stat .record_f64(f);
            running_float_stat.record_f64(f);

            time_window_stat.record_event();
            running_time_stat.record_event();
        }

        let i_max   = upper - 1;
        let f_max   = i_max as f64;

        let i_min   = lower;
        let f_min   = i_min as f64;

        assert!(window_stat.max_i64()        == i_max);
        assert!(running_stat.min_i64()       == i_min);

        assert!(float_window_stat.max_f64()  == f_max);
        assert!(running_float_stat.min_f64() == f_min);

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
        drop(running_float_stat);
        drop(float_window_stat);

        set.print();

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** rc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 7);
        assert!(traverser.sets == 2);

        // Add more subsets to test removal operations.

        let subset_1 = set.add_subset("subset 1", 4, 4);
        let subset_2 = set.add_subset("subset 2", 4, 4);

        add_stats(&mut rc_item_mut!(*subset_1));
        add_stats(&mut rc_item_mut!(*subset_2));

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

        let _ = set.add_counter("No Units", None);
    }

    fn new_hier_integer() -> Hier {
        crate::hier::tests::make_hier(4, 8)
    }

    fn sample_usage() {
        // The last two parameters to new() and add_subset are size hints.
        // They are only hints.
        //
        //  Create the parent set and add a subset.

        let mut set     = RcSet::new("sample usage parent", 0, 0, &None);
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

        timer_mut!(*timer).start();

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

        let member = RunningInteger::new("added as member", &None);
        let member = rc_box!(member);
        set.add_member(member);

        set.print();

        let hier_integer = new_hier_integer();
        let member       = rc_box!(hier_integer);

        set.add_member(member);

        set.print();
    }

    fn test_hier() {
        let     auto_next      = 1000;
        let mut set            = RcSet::new("Hier Test", 0, 0, &None);

        let     integer_name   = "Integer Test";
        let     time_name      = "Time Test";
        let     float_name     = "Float Test";

        let     integer_config = make_integer_config(integer_name, auto_next, None);
        let     time_config    = make_time_config   (time_name,    auto_next, None);
        let     float_config   = make_float_config  (float_name,   auto_next, None);

        let     integer_hier   = set.add_integer_hier(integer_config);
        let     time_hier      = set.add_time_hier   (time_config   );
        let     float_hier     = set.add_float_hier  (float_config  );

        let mut integer_stat = integer_hier.borrow_mut();
        let mut time_stat    = time_hier.   borrow_mut();
        let mut float_stat   = float_hier.  borrow_mut();

        // Fill the first level 0 Rustics instance in each of the
        // Hier instances and check the values recorded.

        let samples = auto_next as i64;

        for i in 1..=samples {
            let f = i as f64;

            integer_stat.record_i64 (i);
            time_stat   .record_time(i);
            float_stat  .record_f64 (f);
        }

        // Now record a partial window and check that we have
        // moved past the old samples.

        let sum  = (samples * (samples + 1)) / 2;
        let mean = sum as f64 / samples as f64;

        assert!(integer_stat.mean()  == mean);
        assert!(time_stat   .mean()  == mean);
        assert!(float_stat  .mean()  == mean);

        assert!(integer_stat.count() == samples as u64);
        assert!(time_stat   .count() == samples as u64);
        assert!(float_stat  .count() == samples as u64);

        let samples = samples / 4;

        for i in 1..=samples {
            let f = i as f64;

            integer_stat.record_i64 (i);
            time_stat   .record_time(i);
            float_stat  .record_f64 (f);
        }

        let sum  = (samples * (samples + 1)) / 2;
        let mean = sum as f64 / samples as f64;

        assert!(integer_stat.mean()  == mean);
        assert!(time_stat   .mean()  == mean);
        assert!(float_stat  .mean()  == mean);

        assert!(integer_stat.count() == samples as u64);
        assert!(time_stat   .count() == samples as u64);
        assert!(float_stat  .count() == samples as u64);

        //  Now check that the total sample counter is correct.

        let event_count = samples + auto_next;

        let integer_generic   = integer_stat.generic();
        let time_generic      = time_stat   .generic();
        let float_generic     = float_stat  .generic();

        let hier_integer_hier = integer_generic.downcast_ref::<Hier>().unwrap();
        let hier_time_hier    = time_generic   .downcast_ref::<Hier>().unwrap();
        let hier_float_hier   = float_generic  .downcast_ref::<Hier>().unwrap();

        assert!(hier_integer_hier.event_count() == event_count);
        assert!(hier_time_hier   .event_count() == event_count);
        assert!(hier_float_hier  .event_count() == event_count);

        // Now drop the locks and print the set.

        drop(integer_stat);
        drop(time_stat   );
        drop(float_stat  );

        println!("test_hier:  Setting the title");
        set.set_title("New Title");
        set.print();
        set.clear();

        let mut float_stat = float_hier.borrow_mut();
        assert!(float_stat.count() == 0);
        float_stat.record_f64(1.0);
        assert!(float_stat.count() == 1);

        drop(float_stat);

        let     subset    = set.add_subset("print_opts Subset", 0, 0);
        let mut locked    = subset.borrow_mut();
        let     member_rc = locked.add_running_integer("print_opts Stat", bytes());
        let mut member    = member_rc.borrow_mut();

        member.record_i64(42);

        drop(member);
        drop(locked);

        set.print_opts(None, Some("print_opts Title"));

        set.set_title("New Top Title");

        set.print_opts(None, None);

        set.clear();

        let member = rc_item!(member_rc);
        assert!(member.count() == 0);
    }

    fn test_rc_printing() {
        let     title          = "Printing Set Title";
        let     print_opts     = title_to_print_option(title);
        let mut set            = RcSet::new("Printing Set",          0, 0, &print_opts);

        let     subset_1       = set.add_subset("Printing Subset 1", 0, 0);
        let     subset_2       = set.add_subset("Printing Subset 2", 0, 0);

        let     set_stat_1     = set.add_running_integer("Set Rustics 1", None);
        let     set_stat_2     = set.add_running_integer("Set Rustics 2", None);

        let mut subset_1_lock  = subset_1.borrow_mut();
        let mut subset_2_lock  = subset_2.borrow_mut();

        let     subset_1_stat  = subset_1_lock.add_running_integer("Subset 1 Rustics", None);
        let     subset_2_stat  = subset_2_lock.add_running_integer("Subset 2 Rustics", None);

        drop(subset_1_lock);
        drop(subset_2_lock);

        let samples = 200;

        for i in 1..=samples {
            let sample = i as i64;

            set_stat_1   .borrow_mut().record_i64(sample    );
            set_stat_2   .borrow_mut().record_i64(sample * 2);
            subset_1_stat.borrow_mut().record_i64(sample * 5);
            subset_2_stat.borrow_mut().record_i64(sample * 7);
        }

        let expected =
            [
                "Printing Set Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2 bytes",
                "    Std Dev          +5.78791 e+1 bytes",
                "    Variance         +3.35000 e+3 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:                72                 0                 0                 0",
                "",
                "Printing Set Title ==> Set Rustics 2",
                "    Count                 200 ",
                "    Minimum                 2 bytes",
                "    Maximum               400 bytes",
                "    Log Mode                9 ",
                "    Mode Value            384 bytes",
                "    Mean             +2.01000 e+2 bytes",
                "    Std Dev          +1.15758 e+2 bytes",
                "    Variance         +1.34000 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 1                 1                 2",
                "    4:                 4                 8                16                32",
                "    8:                64                72                 0                 0",
                "",
                "Printing Set Title ==> Printing Subset 1 ==> Subset 1 Rustics",
                "    Count                 200 ",
                "    Minimum                 5 bytes",
                "    Maximum             1,000 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +5.02500 e+2 bytes",
                "    Std Dev          +2.89395 e+2 bytes",
                "    Variance         +8.37500 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 2                 3                 6                13",
                "    8:                26                51                98                 0",
                "",
                "Printing Set Title ==> Printing Subset 2 ==> Subset 2 Rustics",
                "    Count                 200 ",
                "    Minimum                 7 bytes",
                "    Maximum             1,400 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +7.03500 e+2 bytes",
                "    Std Dev          +4.05154 e+2 bytes",
                "    Variance         +1.64150 e+5 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        let printer = check_printer_box(&expected, true, false);

        set.print_opts(Some(printer.clone()), None);

        println!("test_rc_printing:  end print 1");
        assert! (check_printer_count_match(printer.clone()));

        rc_item_mut!(subset_2).set_title("New Subset 2");

        let expected =
            [
                "Printing Set Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2 bytes",
                "    Std Dev          +5.78791 e+1 bytes",
                "    Variance         +3.35000 e+3 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:                72                 0                 0                 0",
                "",
                "Printing Set Title ==> Set Rustics 2",
                "    Count                 200 ",
                "    Minimum                 2 bytes",
                "    Maximum               400 bytes",
                "    Log Mode                9 ",
                "    Mode Value            384 bytes",
                "    Mean             +2.01000 e+2 bytes",
                "    Std Dev          +1.15758 e+2 bytes",
                "    Variance         +1.34000 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 1                 1                 2",
                "    4:                 4                 8                16                32",
                "    8:                64                72                 0                 0",
                "",
                "Printing Set Title ==> Printing Subset 1 ==> Subset 1 Rustics",
                "    Count                 200 ",
                "    Minimum                 5 bytes",
                "    Maximum             1,000 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +5.02500 e+2 bytes",
                "    Std Dev          +2.89395 e+2 bytes",
                "    Variance         +8.37500 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 2                 3                 6                13",
                "    8:                26                51                98                 0",
                "",
                "New Subset 2 ==> Subset 2 Rustics",
                "    Count                 200 ",
                "    Minimum                 7 bytes",
                "    Maximum             1,400 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +7.03500 e+2 bytes",
                "    Std Dev          +4.05154 e+2 bytes",
                "    Variance         +1.64150 e+5 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                "",
            ];

        let printer = check_printer_box(&expected, true, false);

        set.print_opts(Some(printer.clone()), None);

        println!("test_rc_printing:  end print 2");
        assert! (check_printer_count_match(printer.clone()));

        let expected =
            [
                "Option Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2 bytes",
                "    Std Dev          +5.78791 e+1 bytes",
                "    Variance         +3.35000 e+3 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:                72                 0                 0                 0",
                "",
                "Option Title ==> Set Rustics 2",
                "    Count                 200 ",
                "    Minimum                 2 bytes",
                "    Maximum               400 bytes",
                "    Log Mode                9 ",
                "    Mode Value            384 bytes",
                "    Mean             +2.01000 e+2 bytes",
                "    Std Dev          +1.15758 e+2 bytes",
                "    Variance         +1.34000 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 1                 1                 2",
                "    4:                 4                 8                16                32",
                "    8:                64                72                 0                 0",
                "",
                "Option Title ==> Printing Subset 1 ==> Subset 1 Rustics",
                "    Count                 200 ",
                "    Minimum                 5 bytes",
                "    Maximum             1,000 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +5.02500 e+2 bytes",
                "    Std Dev          +2.89395 e+2 bytes",
                "    Variance         +8.37500 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 2                 3                 6                13",
                "    8:                26                51                98                 0",
                "",
                "Option Title ==> Printing Subset 2 ==> Subset 2 Rustics",
                "    Count                 200 ",
                "    Minimum                 7 bytes",
                "    Maximum             1,400 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +7.03500 e+2 bytes",
                "    Std Dev          +4.05154 e+2 bytes",
                "    Variance         +1.64150 e+5 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        let title    = "Option Title";
        let printer  = check_printer_box(&expected, true, false);

        set.print_opts(Some(printer.clone()), Some(title));

        println!("test_rc_printing:  end print 3");
        assert! (check_printer_count_match(printer.clone()));

        let expected =
            [
                "Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2 bytes",
                "    Std Dev          +5.78791 e+1 bytes",
                "    Variance         +3.35000 e+3 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 1                 1                 2                 4",
                "    4:                 8                16                32                64",
                "    8:                72                 0                 0                 0",
                "",
                "Set Rustics 2",
                "    Count                 200 ",
                "    Minimum                 2 bytes",
                "    Maximum               400 bytes",
                "    Log Mode                9 ",
                "    Mode Value            384 bytes",
                "    Mean             +2.01000 e+2 bytes",
                "    Std Dev          +1.15758 e+2 bytes",
                "    Variance         +1.34000 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 1                 1                 2",
                "    4:                 4                 8                16                32",
                "    8:                64                72                 0                 0",
                "",
                "Printing Subset 1 ==> Subset 1 Rustics",
                "    Count                 200 ",
                "    Minimum                 5 bytes",
                "    Maximum             1,000 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +5.02500 e+2 bytes",
                "    Std Dev          +2.89395 e+2 bytes",
                "    Variance         +8.37500 e+4 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 2                 3                 6                13",
                "    8:                26                51                98                 0",
                "",
                "Printing Subset 2 ==> Subset 2 Rustics",
                "    Count                 200 ",
                "    Minimum                 7 bytes",
                "    Maximum             1,400 bytes",
                "    Log Mode               10 ",
                "    Mode Value            768 bytes",
                "    Mean             +7.03500 e+2 bytes",
                "    Std Dev          +4.05154 e+2 bytes",
                "    Variance         +1.64150 e+5 ",
                "    Skewness         -2.61784 e-8 ",
                "    Kurtosis         -1.19992 e+0 ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        println!("test_arc_printing:  start print 4");

        let printer  = check_printer_box(&expected, true, false);

        set.set_title("");
        set.print_opts(Some(printer.clone()), None);

        let (current, total) = check_printer_counters(printer.clone());

        println!("test_arc_printing:  end print 4");
        println!("test_arc_printing:  print 4:  {} vs {}", current, total);

        assert!(check_printer_count_match(printer.clone()));
    }

    #[test]
    pub fn run_tests() {
        simple_test();
        sample_usage();
        test_hier();
        test_rc_printing();
    }
}
