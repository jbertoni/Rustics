//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkeley 3-clause,
//  and MIT licenses.
//

//!
//! ## Type
//!
//! * ArcSet
//!     * ArcSet implements collections that can contain Rustics instances and
//!       other ArcSet instances.
//!
//!     * Members of an ArcSet are kept as `Arc<Mutex<...>>` instances to support
//!       multithreaded applications.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use std::time::Instant;
//!    use rustics::arc_item_mut;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::arc_sets::ArcSet;
//!    use rustics::timer;
//!    use rustics::timer_mut;
//!
//!    // Create a set.  By way of example, assume that we're expecting
//!    // 8 Rustics instances but no subsets, and set those hints
//!    // appropriately.  By default, the print output goes to stdout, and
//!    // that's fine for an example, so just give "None" to accept the
//!    // default output settings.
//!
//!    let set = ArcSet::new_box("Main Statistics", 8, 0, &None);
//!    let set = arc_item_mut!(set);
//!
//!    // Add an instance to record query latencies.  It's a time
//!    // statistic, so we need a timer.  Here we use an adapter for the
//!    // rust standard Duration timer.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    // The add_running_timer() method is a helper method for creating
//!    // RunningTime instances.  Clone the timer so that we have a copy
//!    // to use.
//!
//!    let mut query_latency =
//!        set.add_running_time("Query Latency", timer.clone());
//!
//!    // Assume for this example that the queries recorded to this
//!    // RunningTime instance are single-threaded, so we can use the
//!    // record_event() method to query the timer and restart it.
//!    //
//!    // The clock started running when we created the DurationTimer.
//!    // Applications also can restart the timer using the start() method
//!    // if more precision is needed.
//!
//!    timer_mut!(timer).start();  // Show how to restart a timer.
//!
//!    arc_item_mut!(query_latency).record_event();
//!
//!    // Do more work, then record another time sample.
//!
//!    // do_work();
//!
//!    // The record_event() code restarted the timer, so we can just
//!    // invoke that routine again.
//!
//!    arc_item_mut!(query_latency).record_event();
//!
//!    // For the multithreaded case, you can use DurationTimer more
//!    // manually.  A timer in a box is required.
//!
//!    let mut local_timer = DurationTimer::new_box();
//!
//!    // Do our query.
//!
//!    // do_work();
//!
//!    // Now record the time spent.  The record_interval() method will
//!    // read the clock for us.
//!
//!    let lock = arc_item_mut!(query_latency);
//!
//!    lock.record_interval(&mut local_timer);
//!
//!    drop(lock);
//!
//!    // If you want to use a timer that can't fully implement the Timer
//!    // trait, you'll need to implement a hz method for Timer with
//!    // dummy functions for the test of the trait.
//!    //
//!    // Let's use Duration timer directly as an example.  Make a new
//!    // Timer instance for this example.  This timer is used only to
//!    // pass the clock hertz to the RunningTimer code.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    // Create a new RunningTime instance.
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
//!    // do_query();
//!
//!    // Now get the elapsed time as integer ticks.  DurationTimer
//!    // works in nanoseconds, so use the as_nanos() method.
//!
//!    assert!(timer!(timer).hz() == 1_000_000_000);
//!    let time_spent = start.elapsed().as_nanos();
//!
//!    arc_item_mut!(query_latency).record_time(time_spent as i64);
//!
//!    // Print our statistics.  This example has very little
//!    // recorded in it.
//!
//!    let query_lock = arc_item_mut!(query_latency);
//!
//!    query_lock.print();
//!
//!    // Check the statistics.
//!
//!    assert!(query_lock.count() == 1);
//!    assert!(query_lock.mean() == time_spent as f64);
//!    assert!(query_lock.standard_deviation() == 0.0);
//!
//!```

use std::sync::Mutex;
use std::sync::Arc;

use super::Rustics;

use super::running_integer::RunningInteger;
use super::running_time::RunningTime;
use super::running_float::RunningFloat;

use super::integer_window::IntegerWindow;
use super::time_window::TimeWindow;
use super::float_window::FloatWindow;

use super::integer_hier::IntegerHier;
use super::integer_hier::IntegerHierConfig;
use super::time_hier::TimeHier;
use super::time_hier::TimeHierConfig;
use super::float_hier::FloatHier;
use super::float_hier::FloatHierConfig;

use super::counter::Counter;
use super::TimerBox;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOpts;
use super::PrintOption;
use super::UnitsOption;
use super::parse_printer;
use super::parse_title;
use super::parse_units;
use super::parse_histo_opts;
use super::make_title;

pub type RusticsArc = Arc<Mutex<dyn Rustics>>;
pub type ArcSetBox  = Arc<Mutex<ArcSet>>;

/// Creates a shareable instance for an ArcSet item.

#[macro_export]
macro_rules! arc_box { ($x:expr) => { Arc::from(Mutex::new($x)) } }

/// Converts an ArcSet item into a mutable Rustics or
/// subset reference.

#[macro_export]
macro_rules! arc_item_mut { ($x:expr) => { &mut *$x.lock().unwrap() } }

/// Converts an ArcSet item into a Rustics or subset
/// reference.

#[macro_export]
macro_rules! arc_item { ($x:expr) => { &*$x.lock().unwrap() } }

/// The ArcTraverser trait is used by the traverse() method to
/// call a user-defined function for each member of an ArcSet
/// and its subsets.

pub trait ArcTraverser {
    /// This method is invoked for each subset in the set and
    /// for the top-level set itself.

    fn visit_set(&mut self, set: &mut ArcSet);

    /// This method is invoked on every Rustics instance
    /// in the set and its subsets.

    fn visit_member(&mut self, member: &mut dyn Rustics);
}

/// ArcSet is the implementation type for a set of Rustics instances
/// that are wrapped as `Arc<Mutex<dyn Rustics>>`.

#[derive(Clone)]
pub struct ArcSet {
    name:       String,
    title:      String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsArc>,
    subsets:    Vec<ArcSetBox>,
    printer:    PrinterBox,
    print_opts: PrintOption,
}

/// This struct is passed to some constructors that create
/// ArcSet instances.

pub struct ArcSetConfig {
    name:          String,
    rustics_hint:  usize,
    subsets_hint:  usize,
    title:         Option<String>,
    id:            usize,
    print_opts:    PrintOption,
}

impl ArcSet {
    /// Creates a new ArcSet.
    ///
    /// The "rustics_hint" and "subsets_hint" parameters are hints as to the number
    /// of Rustics instances and subset to be expected.

    pub fn new(name: &str, rustics_hint: usize, subsets_hint: usize, print_opts: &PrintOption)
            -> ArcSet {
        let name       = name.to_string();
        let id         = usize::MAX;
        let print_opts = print_opts.clone();
        let title      = None;

        let configuration =
            ArcSetConfig { name, rustics_hint, subsets_hint, title, id, print_opts };

        ArcSet::new_from_config(configuration)
    }

    /// Creates a new ArcSetBox (an `Arc<Mutex<ArcSet>>`).

    pub fn new_box(name: &str, rustics_hint: usize, subsets_hint: usize, print_opts: &PrintOption)
            -> ArcSetBox {
        let arc_set = ArcSet::new(name, rustics_hint, subsets_hint, print_opts);

        arc_box!(arc_set)
    }

    /// Creates a new ArcSet given a configuration.

    pub fn new_from_config(configuration: ArcSetConfig) -> ArcSet {
        let name       = configuration.name;
        let print_opts = configuration.print_opts;
        let title      = configuration.title;
        let id         = configuration.id;
        let next_id    = 1;
        let members    = Vec::with_capacity(configuration.rustics_hint);
        let subsets    = Vec::with_capacity(configuration.subsets_hint);
        let printer    = parse_printer(&print_opts);

        let title =
            if let Some(title) = title {
                title
            } else {
                parse_title(&print_opts, &name)
            };

        ArcSet { name, title, id, next_id, members, subsets, printer, print_opts }
    }

    /// Creates a new ArcSetBox given a configuration.

    pub fn new_box_from_config(configuration: ArcSetConfig) -> ArcSetBox {
        let subset = ArcSet::new_from_config(configuration);

        arc_box!(subset)
    }

    /// Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Traverses the Rustics instances and subsets in the set invoking a
    /// user-supplied callback for each member.

    pub fn traverse(&mut self, traverser: &mut dyn ArcTraverser) {
        traverser.visit_set(self);

        for mutex in self.members.iter() {
            let member = arc_item_mut!(mutex);

            traverser.visit_member(&mut *member);
        }

        for mutex in self.subsets.iter() {
            let subset = arc_item_mut!(mutex);

            subset.traverse(traverser);
        }
    }

    /// Prints the set and all its constituents (subsets and Rustics
    /// instances).

    pub fn print(&self) {
        self.print_opts(None, None);
    }

    /// Prints the set and overrides the default printer and title as
    /// desired.

    pub fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        // Iterate through the Rustics instances.

        for mutex in self.members.iter() {
            let member  = arc_item_mut!(mutex);
            let printer = printer.clone();

            if let Some(title) = title {
                let title = make_title(title, &member.name());
                let title = Some(title.as_str());

                member.print_opts(printer, title);
            } else {
                member.print_opts(printer, None);
            }
        }

        // Iterate through the subsets.

        for mutex in self.subsets.iter() {
            let subset  = arc_item_mut!(mutex);
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

    /// Returns the current title.

    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// Sets the title for the ArcSet.

    pub fn set_title(&mut self, title: &str) {
        self.title = String::from(title);

        for mutex in self.subsets.iter() {
            let subset = arc_item_mut!(mutex);
            let title  = make_title(title, &subset.name());

            subset.set_title(&title);
        }

        for mutex in self.members.iter() {
            let member = arc_item_mut!(mutex);
            let title  = make_title(title, &member.name());

            member.set_title(&title);
        }
    }

    /// Does a recursive clear of all Rustics instances in the set
    /// and its entire subset hierarchy.

    pub fn clear(&mut self) {
        for mutex in self.subsets.iter() {
            let subset = arc_item_mut!(mutex);

            subset.clear();
        }

        for mutex in self.members.iter() {
            let member = arc_item_mut!(mutex);

            member.clear();
        }
    }

    /// Adds a RusticsArc instance to a set.  The user creates the
    /// Rustics instance and passes it in an Arc.  This is
    /// a bit more manual than add_running_integer() and similar
    /// methods.

    pub fn add_member(&mut self, member: RusticsArc) {
        let work  = member.clone();
        let stat  = arc_item_mut!(work);
        let title = make_title(&self.title, &stat.name());

        stat.set_title(&title);
        stat.set_id(self.next_id);
        self.next_id += 1;

        self.members.push(member);
    }

    /// Creates a RunningInteger instance and adds it to the set.

    pub fn add_running_integer(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let mut member = RunningInteger::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates an IntegerWindow instance and adds it to the set.

    pub fn add_integer_window(&mut self, name: &str, window_size: usize, units: UnitsOption)
            -> RusticsArc {
        let mut member = IntegerWindow::new(name, window_size, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningInteger as the base type and adds it to the set.

    pub fn add_integer_hier(&mut self, mut configuration: IntegerHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = IntegerHier::new_hier(configuration);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningTime instance and adds it to the set.  The user
    /// must provide a timer.  The timer can be used with the record_event
    /// method and is queried by the print routines to determine the hertz
    /// for the samples.

    pub fn add_running_time(&mut self, name: &str, timer: TimerBox) -> RusticsArc {
        let member = RunningTime::new(name, timer, &self.print_opts);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a TimeWindow instance and adds it to the set.

    pub fn add_time_window(&mut self, name: &str, window_size: usize, timer: TimerBox)
            -> RusticsArc {
        let member = TimeWindow::new(name, window_size, timer, &self.print_opts);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningTime as the base type and adds it to the set.

    pub fn add_time_hier(&mut self, mut configuration: TimeHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = TimeHier::new_hier(configuration);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningFloat instance and adds it to the set.

    pub fn add_running_float(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let mut member = RunningFloat::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a FloatWindow instance and adds it to the set.

    pub fn add_float_window(&mut self, name: &str, window_size: usize, units: UnitsOption)
            -> RusticsArc {
        let mut member = FloatWindow::new(name, window_size, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningFloat as the base type and adds it to the set.

    pub fn add_float_hier(&mut self, mut configuration: FloatHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = FloatHier::new_hier(configuration);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    /// Creates a Counter instance and adds it to the set.

    pub fn add_counter(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let printer    = Some(self.printer.clone());
        let title      = None;
        let histo_opts = None;

        let print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let member = Counter::new(name, &print_opts);
        let member = arc_box!(member);

        self.add_member(member.clone());
        member
    }

    // Merge the input print_ops with the title that we generate and the printer
    // for the set.

    fn make_print_opts(&self, name: &str, print_opts: &PrintOption) -> PrintOption {
        let     printer    = Some(self.printer.clone());
        let     title      = Some(make_title(&self.title, name));
        let     units      = Some(parse_units(print_opts));
        let     histo_opts = Some(parse_histo_opts(print_opts));
        let     print_opts = PrintOpts { printer, title, units, histo_opts };

        Some(print_opts)
    }

    /// Removes a Rustics instance from the set.

    pub fn remove_stat(&mut self, target_box: RusticsArc) -> bool {
        let mut found       = false;
        let mut i           = 0;
        let     target_stat = target_box.lock().unwrap(); // can't use arc_item!
        let     target_id   = target_stat.id();

        // We have to unlock the target_box or we'll hang in the loop.

        drop(target_stat);

        for mutex in self.members.iter() {
            let stat = arc_item!(mutex);

            found = stat.id() == target_id;

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

    pub fn add_subset(&mut self, name: &str, rustics_hint: usize, subsets_hint: usize)
            -> ArcSetBox {
        let name       = name.to_string();
        let title      = Some(make_title(&self.title, &name));
        let id         = self.next_id;
        let print_opts = self.print_opts.clone();

        let configuration =
            ArcSetConfig { name, rustics_hint, subsets_hint, title, id, print_opts };

        let subset = ArcSet::new_box_from_config(configuration);

        self.next_id += 1;
        self.subsets.push(subset.clone());
        subset
    }

    /// Removes a subset from the set.

    pub fn remove_subset(&mut self, target_box: ArcSetBox) -> bool {
        let mut found         = false;
        let mut i             = 0;
        let     target_subset = target_box.lock().unwrap();
        let     target_id     = target_subset.id();

        // We have to unlock the target_box or we'll hang in the loop.

        drop(target_subset);

        for mutex in self.subsets.iter() {
            let subset = arc_item!(mutex);

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

    // The following method is for internal use only.

    fn id(&self) -> usize {
        self.id
    }
}

#[cfg(test)]
pub mod tests {
    use std::time::Instant;

    use super::*;
    use crate::tests::TestTimer;
    use crate::tests::ConverterTrait;
    use crate::tests::continuing_box;
    use crate::tests::check_printer_box;
    use crate::tests::check_printer_count_match;
    use crate::tests::check_printer_counters;
    use crate::tests::bytes;
    use crate::hier::Hier;
    use crate::time::Timer;
    use crate::time::DurationTimer;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;
    use crate::stdout_printer;

    struct TestTraverser {
        pub members:  i64,
        pub sets:     i64,
    }

    impl TestTraverser {
        pub fn new() -> TestTraverser {
            println!(" *** making an arc traverser");
            TestTraverser { members:  0, sets:  0 }
        }
    }

    impl ArcTraverser for TestTraverser {
        fn visit_member(&mut self, member: &mut dyn Rustics) {
            println!(" *** visiting arc member  \"{}\"", member.name());
            self.members += 1;
        }

        fn visit_set(&mut self, set: &mut ArcSet) {
            println!(" *** visiting arc set     \"{}\"", set.name());
            self.sets += 1;
        }
    }

    //  Add Rustics instances to a set.

    fn add_stats(parent: &Mutex<ArcSet>) {
        for i in 0..4 {
            let lower             = -64;    // Just define the range for the test samples.
            let upper             =  64;
            let events_limit      = 2 * (upper - lower) as usize;

            let parent            = &mut arc_item_mut!(parent);
            let subset_name       = format!("generated subset {}", i);
            let subset            = parent.add_subset(&subset_name, 4, 4);
            let subset            = &mut arc_item_mut!(subset);

            let window_name       = format!("generated window {}", i);
            let running_name      = format!("generated running {}", i);
            let window_mutex      = subset.add_integer_window(&window_name, events_limit, bytes());
            let running_mutex     = subset.add_running_integer(&running_name, None);

            let window            = arc_item_mut!(window_mutex);
            let running           = arc_item_mut!(running_mutex);

            let subset_expected   = make_title(&parent.title(),  &subset_name );
            let window_expected   = make_title(&subset_expected, &window_name );
            let running_expected  = make_title(&subset_expected, &running_name);

            assert!(subset.title()  == subset_expected );
            assert!(window.title()  == window_expected );
            assert!(running.title() == running_expected);

            // Record some events and see how that goes.

            let mut events = 0;

            for i in lower..=upper {
                window .record_i64(i);
                running.record_i64(i);

                events += 1;
            }

            // Compute the expected mean for the stats.

            let mean = (((upper + lower) as f64) / 2.0) / events as f64;

            assert!(running.mean()  == mean  );
            assert!(window.mean()   == mean  );
            assert!(running.count() == events);
            assert!(window.count()  == events);
        }
    }

    pub fn simple_test() {
        let lower       = -32;
        let upper       =  32;
        let test_hz     = 1_000_000_000;
        let parent_name = "parent set";

        //  Create the parent set for our test Rustics instances.

        let set = ArcSet::new_box(&parent_name, 4, 4, &None);
        let set = arc_item_mut!(set);

        //  Create timers for time statistics.

        let window_timer  = continuing_box();
        let running_timer = continuing_box();

        //  Now create the instances in our set.

        let window_size = 32;

        let window_mutex        = set.add_integer_window ("window",        window_size, bytes()      );
        let running_mutex       = set.add_running_integer("running",                    bytes()      );
        let time_window_mutex   = set.add_time_window    ("time window",   window_size, window_timer );
        let running_time_mutex  = set.add_running_time   ("running time",               running_timer);
        let float_window_mutex  = set.add_float_window   ("float window",  window_size, bytes()      );
        let running_float_mutex = set.add_running_float  ("running float",              bytes()      );

        // Lock the instances for manipulation.  arc_item_mut doesn't
        // interact well with drop...

        let mut window          = window_mutex       .lock().unwrap();
        let mut running         = running_mutex      .lock().unwrap();
        let mut time_window     = time_window_mutex  .lock().unwrap();
        let mut running_time    = running_time_mutex .lock().unwrap();
        let mut float_window    = float_window_mutex .lock().unwrap();
        let mut running_float   = running_float_mutex.lock().unwrap();

        //  Create some simple timers to be started manually.

        let     running_both    = TestTimer::new_box(test_hz);

        let     running_test    = ConverterTrait::as_test_timer(running_both.clone());
        let mut running_stat    = ConverterTrait::as_timer     (running_both.clone());

        let     window_both     = TestTimer::new_box(test_hz);

        let     window_test     = ConverterTrait::as_test_timer(window_both.clone());
        let mut window_stat     = ConverterTrait::as_timer     (window_both.clone());

        //  Now record some data in all the instances.

        for i in lower..upper {
            let f = i as f64;

            window       .record_i64(i);
            running      .record_i64(i);
            running_float.record_f64(f);
            float_window .record_f64(f);

            assert!(window       .max_i64() == i);
            assert!(running      .max_i64() == i);
            assert!(running_float.max_f64() == f);
            assert!(float_window .max_f64() == f);

            // Get a test value to use.  It must be positive.

            let expected = 10 + (i + -lower) * 10;

            running_test.borrow_mut().setup(expected);
            running_time.record_interval(&mut running_stat);

            // Now this value should set the max.  See what happened.

            let elapsed = running_time.max_i64();

            assert!(running_time.max_i64() == elapsed);

            // Try this with the window.

            let expected = 100 + (i + -lower) * 100;

            window_test.borrow_mut().setup(expected);
            time_window.record_interval(&mut window_stat);

            assert!(time_window.max_i64() == expected);
        }

        //  Make sure the titles are being created properly.

        let set_title = set.title();

        assert!(set_title            == parent_name);
        assert!(running_time.title() == make_title(&"parent set", &"running time"));
        assert!(time_window.title()  == make_title(&"parent set", &"time window" ));
        assert!(running.title()      == make_title(&"parent set", &"running"     ));
        assert!(window.title()       == make_title(&"parent set", &"window"      ));

        //  Create a subset to check titles in a subtree.

        let     subset      = set.add_subset("subset", 0, 0);
        let mut subset      = subset.lock().unwrap();
        let     subset_stat = subset.add_running_integer("subset stat", None);
        let     subset_stat = subset_stat.lock().unwrap();

        assert!(subset.title()      == make_title(&set_title, "subset"));
        assert!(subset_stat.title() == make_title(&subset.title(), &"subset stat"));

        //  Drop all the locks.

        drop(subset       );
        drop(subset_stat  );
        drop(window       );
        drop(running      );
        drop(running_time );
        drop(time_window  );
        drop(running_float);
        drop(float_window );

        //  Make sure that print completes.

        set.print();

        //  Do a test of the traverser.  Check that we see the correct
        //  number of members and subsets.

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** arc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 7);
        assert!(traverser.sets    == 2);

        //  Now test removing Rustics instances.

        let subset_1_name = "subset 1";
        let subset_2_name = "subset 2";
        let subset_1      = set.add_subset(subset_1_name, 4, 4);
        let subset_2      = set.add_subset(subset_2_name, 4, 4);

        let subset_1_impl = subset_1.lock().unwrap();
        let subset_2_impl = subset_2.lock().unwrap();

        assert!(subset_1_impl.title() == make_title(parent_name, &subset_1_name));
        assert!(subset_2_impl.title() == make_title(parent_name, &subset_2_name));

        drop(subset_1_impl);
        drop(subset_2_impl);

        add_stats(&subset_1);
        add_stats(&subset_2);

        set.print_opts(Some(stdout_printer()), Some("print_opts Test"));

        // Before testing remove operations, traverse again...

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** arc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 23);
        assert!(traverser.sets    == 12);

        // Print the set, as well.

        set.print();

        // Remove a subset and check that it goes away.

        let found = set.remove_subset(subset_1.clone());
        assert!(found);

        let found = set.remove_subset(subset_1);
        assert!(!found);

        // Remove two stats and check that they go away.
        //
        // First, do the remove operations.

        let found = set.remove_stat(window_mutex.clone());
        assert!(found);

        let found = set.remove_stat(running_mutex.clone());
        assert!(found);

        // Now check that the stats went away

        let found = set.remove_stat(window_mutex);
        assert!(!found);

        let found = set.remove_stat(running_mutex);
        assert!(!found);
    }


    fn new_hier() -> Hier {
        crate::hier::tests::make_hier(4, 8)
    }

    fn sample_usage() {
        // The last two parameters to new() are size hints, and need not be correct.
        // The same is true for add_subset.

        let      set     = ArcSet::new_box("parent set", 0, 1, &None);
        let mut  set     = set    .lock().unwrap();
        let      subset  = set    .add_subset("subset", 1, 0);
        let mut  subset  = subset .lock().unwrap();
        let      running = subset .add_running_integer("running", None);
        let mut  running = running.lock().unwrap();

        for i in 0..64 {
            running.record_i64(i);
        }

        //  Drop the locks before trying to print.

        drop(running);
        drop(subset);

        let printer = stdout_printer();

        set.print_opts(Some(printer.clone()), None);

        // Add a counter.

        let     counter_arc = set.add_counter("test counter", None);
        let mut counter     = counter_arc.lock().unwrap();
        let     limit       = 20;

        for _i in 1..=limit {
            counter.record_event();    // increment by 1
            counter.record_i64(1);     // increment by 1
        }

        //  Check the counter value.

        assert!(counter.count() == 2 * limit as u64);

        //  Drop the lock before printing.

        drop(counter);

        //  print should still work.

        let member = RunningInteger::new("added as member", &None);
        let member = arc_box!(member);

        set.add_member(member);

        set.print_opts(Some(printer.clone()), None);

        // Try adding a hierarchical Rustics instance.

        let hier_integer = new_hier();
        let member       = arc_box!(hier_integer);

        set.add_member(member);

        set.print();
    }

    fn documentation() {
       // Create a set.  We're expecting 8 Rustics instances but
       // no subsets, so we set those hints appropriately.  The
       // default print output goes to stdout, and that's fine for
       // an example, so just give "None" to accept the default.
       // See the Printer trait to implement a custom printer.

       let     set = ArcSet::new_box("Main Statistics", 8, 0, &None);
       let mut set = set.lock().unwrap();

       // Add an instance to record query latencies.  It's a time
       // statistic, so we need a timer.  Use an adapter for the
       // rust standard Duration timer.  The add_running_timer
       // function is a help for creating RunningTime instances.

       let timer = DurationTimer::new_box();

       let query_latency = set.add_running_time("Query Latency", timer);

       // By way of example, we assume that the queries are single-
       // threaded, so we can use the record_time() method to
       // query the timer and restart it.  Multi-threaded apps will
       // need to use record_interval and manage the clocks themselves.
       // if they want to share a single RunningTime instance.
       //
       // So record one event time for the single-threaded case.

       query_latency.lock().unwrap().record_event();

       // For the multithreaded case, you can use DurationTimer manually.

       let mut local_timer = DurationTimer::new();

       // Do our query.
       // ...

       let mut lock = query_latency.lock().unwrap();

       lock.record_time(local_timer.finish() as i64);

       drop(lock);

       // If you want to use your own timer, you'll need to implement
       // the Timer trait to initialize the RunningTime instance, but you
       //can use it directly to get data. Let's use Duration timer directly
       // as an example.  Make a new instance for this example.

       let timer = DurationTimer::new_box();

       let query_latency = set.add_running_time("Custom Timer Query Latency", timer);

       // Start the Duration timer.

       let start = Instant::now();

       // Do our query.

       // Now get the elapsed time.  DurationTimer works in nanoseconds,
       // so use as_nanos().

       let time_spent = start.elapsed().as_nanos();

       query_latency.lock().unwrap().record_time(time_spent as i64);

       // Print our statistics.  This example has only one event recorded.

       let query_lock = query_latency.lock().unwrap();

       query_lock.print();

       assert!(query_lock.count() == 1);
       assert!(query_lock.mean() == time_spent as f64);
       assert!(query_lock.standard_deviation() == 0.0);
    }

    // These routines are used by RcSet tests.

    pub fn level_0_period() -> usize {
        100
    }

    pub fn level_0_retain() -> usize {
        3 * level_0_period()
    }

    pub fn make_descriptor(auto_next: i64) -> HierDescriptor {
        let     levels         = 4;
        let     level_0_period = level_0_period();
        let     dimension      = HierDimension::new(level_0_period, level_0_retain());
        let mut dimensions     = Vec::<HierDimension>::with_capacity(levels);

        // Push the level 0 descriptor.

        dimensions.push(dimension);

        // Create a hierarchy.

        let mut period = 4;

        for _i in 1..levels {
            let dimension = HierDimension::new(period, 3 * period);

            dimensions.push(dimension);

            period += 2;
        }

        HierDescriptor::new(dimensions, Some(auto_next))
    }

    pub fn make_integer_config(name: &str, auto_next: i64, window_size: Option<usize>)
            -> IntegerHierConfig {
        let name       = name.to_string();
        let descriptor = make_descriptor(auto_next);
        let print_opts = None;

        IntegerHierConfig { name, descriptor, print_opts, window_size }
    }

    pub fn make_time_config(name: &str, auto_next: i64, window_size: Option<usize>)
            -> TimeHierConfig {
        let name       = name.to_string();
        let descriptor = make_descriptor(auto_next);
        let print_opts = None;
        let hz         = 1_000_000_000;
        let timer      = TestTimer::new_box(hz);

        TimeHierConfig { name, descriptor, timer, print_opts, window_size }
    }

    pub fn make_float_config(name: &str, auto_next: i64, window_size: Option<usize>)
            -> FloatHierConfig {
        let name       = name.to_string();
        let descriptor = make_descriptor(auto_next);
        let print_opts = None;

        FloatHierConfig { name, descriptor, print_opts, window_size }
    }

    fn test_hier() {
        let     auto_next      = 1000;
        let mut set            = ArcSet::new("Hier Test", 0, 0, &None);
        let     subset         = set.add_subset("Hier Subset", 0, 0);
        let mut subset         = subset.lock().unwrap();
        let     subset_member  = subset.add_running_integer("Subset Member", bytes());

        let     integer_name   = "Integer Test";
        let     time_name      = "Time Test";
        let     float_name     = "Float Test";

        let     integer_config = make_integer_config(integer_name, auto_next, None);
        let     time_config    = make_time_config   (time_name,    auto_next, None);
        let     float_config   = make_float_config  (float_name,   auto_next, None);

        let     integer_hier   = set.add_integer_hier(integer_config);
        let     time_hier      = set.add_time_hier   (time_config   );
        let     float_hier     = set.add_float_hier  (float_config  );

        let mut integer_stat   = integer_hier.lock ().unwrap();
        let mut time_stat      = time_hier.lock    ().unwrap();
        let mut float_stat     = float_hier.lock   ().unwrap();
        let mut subset_stat    = subset_member.lock().unwrap();

        drop(subset);

        // Fill the first level 0 Rustics instance in each of the
        // Hier instances and check the values recorded.

        let samples = auto_next as i64;

        for i in 1..=samples {
            let f = i as f64;

            integer_stat.record_i64 (i);
            time_stat   .record_time(i);
            float_stat  .record_f64 (f);
            subset_stat .record_i64 (i);
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

        let integer_generic = integer_stat.generic();
        let time_generic    = time_stat   .generic();
        let float_generic   = float_stat  .generic();

        let hier_integer_hier    = integer_generic.downcast_ref::<Hier>().unwrap();
        let hier_time_hier       = time_generic   .downcast_ref::<Hier>().unwrap();
        let hier_float_hier      = float_generic  .downcast_ref::<Hier>().unwrap();

        assert!(hier_integer_hier.event_count() == event_count);
        assert!(hier_time_hier   .event_count() == event_count);
        assert!(hier_float_hier  .event_count() == event_count);

        // Now drop the locks and print the set with a new title.

        drop(integer_stat);
        drop(time_stat   );
        drop(float_stat  );
        drop(subset_stat );

        set.set_title("New Title");
        set.print();
        set.clear();

        let mut float_stat      = float_hier.lock().unwrap();
        let     float_generic   = float_stat.generic();
        let     hier_float_hier = float_generic.downcast_ref::<Hier>().unwrap();

        assert!(float_stat.count() == 0);
        assert!(hier_float_hier.event_count() == 0);

        float_stat.record_f64(1.0);

        assert!(float_stat.count() == 1);

        drop(float_stat);

        let float_stat      = float_hier.lock  ().unwrap();
        let float_generic   = float_stat.generic();
        let hier_float_hier = float_generic.downcast_ref::<Hier>().unwrap();

        assert!(hier_float_hier.event_count() == 1);

        drop(float_stat);

        set.print_opts(Some(stdout_printer()), Some("Option Title"));
    }

    pub fn title_to_print_option(title: &str) -> PrintOption {
        let printer    = None;
        let title      = Some(title.to_string());
        let histo_opts = None;
        let units      = bytes();

        Some(PrintOpts { printer, title, histo_opts, units })
    }

    fn test_arc_printing() {
        let     title          = "Printing Set Title";
        let     print_opts     = title_to_print_option(title);
        let mut set            = ArcSet::new("Printing Set",          0, 0, &print_opts);

        let     subset_1       = set.add_subset("Printing Subset 1", 0, 0);
        let     subset_2       = set.add_subset("Printing Subset 2", 0, 0);

        let     set_stat_1     = set.add_running_integer("Set Rustics 1", None);
        let     set_stat_2     = set.add_running_integer("Set Rustics 2", None);

        let mut subset_1_lock  = subset_1.lock().unwrap();
        let mut subset_2_lock  = subset_2.lock().unwrap();

        let     subset_1_stat  = subset_1_lock.add_running_integer("Subset 1 Rustics", None);
        let     subset_2_stat  = subset_2_lock.add_running_integer("Subset 2 Rustics", None);

        drop(subset_1_lock);
        drop(subset_2_lock);

        let samples = 200;

        for i in 1..=samples {
            let sample = i as i64;

            set_stat_1   .lock().unwrap().record_i64(sample    );
            set_stat_2   .lock().unwrap().record_i64(sample * 2);
            subset_1_stat.lock().unwrap().record_i64(sample * 5);
            subset_2_stat.lock().unwrap().record_i64(sample * 7);
        }

        let expected =
            [
                "Printing Set Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2  bytes",
                "    Std Dev          +5.78791 e+1  bytes",
                "    Variance         +3.35000 e+3  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +2.01000 e+2  bytes",
                "    Std Dev          +1.15758 e+2  bytes",
                "    Variance         +1.34000 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +5.02500 e+2  bytes",
                "    Std Dev          +2.89395 e+2  bytes",
                "    Variance         +8.37500 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +7.03500 e+2  bytes",
                "    Std Dev          +4.05154 e+2  bytes",
                "    Variance         +1.64150 e+5  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        println!("test_arc_printing:  start print 1");

        let printer = check_printer_box(&expected, true, false);

        set.print_opts(Some(printer.clone()), None);

        // Check that the output length was an exact match.

        let (current, total) = check_printer_counters(printer.clone());

        println!("test_arc_printing:  end print 1");
        println!("test_arc_printing:  print 1:  {} vs {}", current, total);

        assert!(check_printer_count_match(printer.clone()));

        let expected =
            [
                "Printing Set Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2  bytes",
                "    Std Dev          +5.78791 e+1  bytes",
                "    Variance         +3.35000 e+3  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +2.01000 e+2  bytes",
                "    Std Dev          +1.15758 e+2  bytes",
                "    Variance         +1.34000 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +5.02500 e+2  bytes",
                "    Std Dev          +2.89395 e+2  bytes",
                "    Variance         +8.37500 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +7.03500 e+2  bytes",
                "    Std Dev          +4.05154 e+2  bytes",
                "    Variance         +1.64150 e+5  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        // Set a new title for a subset and check the output.

        let printer = check_printer_box(&expected, true, false);

        subset_2.lock().unwrap().set_title("New Subset 2");
        println!("test_arc_printing:  start print 2");

        set.print_opts(Some(printer.clone()), None);

        let (current, total) = check_printer_counters(printer.clone());

        println!("test_arc_printing:  end print 2");
        println!("test_arc_printing:  print 2:  {} vs {}", current, total);

        assert! (check_printer_count_match(printer.clone()));

        let expected =
            [
                "Option Title ==> Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2  bytes",
                "    Std Dev          +5.78791 e+1  bytes",
                "    Variance         +3.35000 e+3  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +2.01000 e+2  bytes",
                "    Std Dev          +1.15758 e+2  bytes",
                "    Variance         +1.34000 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +5.02500 e+2  bytes",
                "    Std Dev          +2.89395 e+2  bytes",
                "    Variance         +8.37500 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +7.03500 e+2  bytes",
                "    Std Dev          +4.05154 e+2  bytes",
                "    Variance         +1.64150 e+5  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        let title   = "Option Title";
        let printer = check_printer_box(&expected, true, false);

        println!("test_arc_printing:  start print 3");

        set.print_opts(Some(printer.clone()), Some(title));

        let (current, total) = check_printer_counters(printer.clone());

        println!("test_arc_printing:  end print 3");
        println!("test_arc_printing:  print 3:  {} vs {}", current, total);

        assert!(check_printer_count_match(printer.clone()));

        let expected =
            [
                "Set Rustics 1",
                "    Count                 200 ",
                "    Minimum                 1 byte",
                "    Maximum               200 bytes",
                "    Log Mode                8 ",
                "    Mode Value            192 bytes",
                "    Mean             +1.00500 e+2  bytes",
                "    Std Dev          +5.78791 e+1  bytes",
                "    Variance         +3.35000 e+3  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +2.01000 e+2  bytes",
                "    Std Dev          +1.15758 e+2  bytes",
                "    Variance         +1.34000 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +5.02500 e+2  bytes",
                "    Std Dev          +2.89395 e+2  bytes",
                "    Variance         +8.37500 e+4  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
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
                "    Mean             +7.03500 e+2  bytes",
                "    Std Dev          +4.05154 e+2  bytes",
                "    Variance         +1.64150 e+5  ",
                "    Skewness         -2.61784 e-8  ",
                "    Kurtosis         -1.19992 e+0  ",
                "  Log Histogram",
                "  -----------------------",
                "    0:                 0                 0                 0                 1",
                "    4:                 1                 2                 5                 9",
                "    8:                18                37                73                54",
                ""
            ];

        println!("test_arc_printing:  start print 4");

        let printer = check_printer_box(&expected, true, false);

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
        documentation();
        test_hier();
        test_arc_printing();
    }
}
