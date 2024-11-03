//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!
//! ## Type
//!
//! * ArcSet
//!     * ArcSet implements a collection that can contain Rustics instances and
//!       other ArcSet instances.
//!
//!     * Members of an ArcSet are kept as Arc<Mutex<...>> instances to support
//!       multithreaded applications.
//!
//! ## Example
//!```
//!    use std::rc::Rc;
//!    use std::cell::RefCell;
//!    use std::time::Instant;
//!    use rustics::time::Timer;
//!    use rustics::time::DurationTimer;
//!    use rustics::arc_sets::ArcSet;
//!
//!    // Create a set.  By way of example, assume that we're expecting
//!    // 8 statistics instances but no subsets, and set those hints
//!    // appropriately.  The default print output goes to stdout, and
//!    // that's fine for an example, so just give "None" to accept the
//!    // default.
//!
//!    let     set = ArcSet::new_box("Main Statistics", 8, 0, &None);
//!    let mut set = set.lock().unwrap();
//!
//!    // Add an instance to record query latencies.  It's a time
//!    // statistic, so we need a timer.  Here we use an adapter for the
//!    // rust standard Duration timer.  Clone a copy so that we can
//!    // use the timer here.
//!
//!    let timer = DurationTimer::new_box();
//!
//!    // The add_running_timer() method is a helper method for creating
//!    // RunningTime instances.
//!
//!    let mut query_latency =
//!        set.add_running_time("Query Latency", timer.clone());
//!
//!    // Assume for this example that the queries recorded to this
//!    // RunningTime instance are single-threaded, so we can use the
//!    // record_event() method to query the timer and restart it.
//!    //
//!    // The clock started running when we created the DurationTimer.
//!    // Applications can reset the start() method as needed.
//!
//!    timer.borrow_mut().start();  // Show how to restart a timer.
//!
//!    query_latency.lock().unwrap().record_event();
//!
//!    // Do more work, then record another time sample.
//!
//!    // do_work();
//!
//!    // The record_event() code restarted the timer, so we can just
//!    // invoke that routine again.
//!
//!    query_latency.lock().unwrap().record_event();
//!
//!    // For the multithreaded case, you can use DurationTimer manually.
//!
//!    let mut local_timer = DurationTimer::new();
//!
//!    // Do our query.
//!
//!    // do_work();
//!
//!    let mut lock = query_latency.lock().unwrap();
//!
//!    lock.record_time(local_timer.finish() as i64);
//!
//!    drop(lock);
//!
//!    // If you want to use your own timer, you'll need to implement the
//!    // Timer trait or SimpleClock and ClockTimer to initialize the
//!    // RunningTime instance, but you can use that timer directly to get
//!    // data. Let's use Duration timer directly as an example.  Make a
//!    // new Timer instance for this example.  This timer is used only to
//!    // pass the clock hertz to the RunningTimer code.
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
//!    // do_query();
//!
//!    // Now get the elapsed time.  DurationTimer works in nanoseconds,
//!    // so use the as_nanos() method.
//!
//!    assert!(timer.borrow().hz() == 1_000_000_000);
//!    let time_spent = start.elapsed().as_nanos();
//!
//!    query_latency.lock().unwrap().record_time(time_spent as i64);
//!
//!    // Print our statistics.  This example has only one event
//!    // recorded.
//!
//!    let query_lock = query_latency.lock().unwrap();
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

/// The ArcTraverser trait is used by the traverse() method to
/// call a user-defined function at each element in an ArcSet
/// and its subsets.

pub trait ArcTraverser {
    /// This method is invoked for each element in the set,
    /// including the top-level set.

    fn visit_set(&mut self, set: &mut ArcSet);

    /// This method is invoked on every statistics instance
    /// in the set.

    fn visit_member(&mut self, member: &mut dyn Rustics);
}

/// ArcSet is the implementation type for a set of Rustics instances
/// wrapped as Arc<Mutex<dyn Rustics>>.

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

/// This struct is passed to new_from_config to create an ArcSetBox.

pub struct ArcSetConfig {
    name:          String,
    members_hint:  usize,
    subsets_hint:  usize,
    title:         Option<String>,
    id:            usize,
    print_opts:    PrintOption,
}

impl ArcSet {
    /// ArcSet Constructors
    ///
    /// The "members_hint" and "subsets_hint" parameters are hints as to the number
    /// of elements to be expected.  "members_hint" refers to the number of Rustics
    /// instances in the set.  These hints can improve performance a bit.  They
    /// might be especially useful in embedded environments.

    /// Creates a new ArcSet.

    pub fn new(name: &str, members_hint: usize, subsets_hint: usize, print_opts: &PrintOption)
            -> ArcSet {
        let name       = name.to_string();
        let id         = usize::MAX;
        let print_opts = print_opts.clone();
        let title      = None;

        let configuration = 
            ArcSetConfig { name, members_hint, subsets_hint, title, id, print_opts };

        ArcSet::new_from_config(configuration)
    }

    /// Creates a new ArcSetBox (an Arc<Mutex<ArcSet>>).

    pub fn new_box(name: &str, members_hint: usize, subsets_hint: usize, print_opts: &PrintOption)
            -> ArcSetBox {
        let arc_set = ArcSet::new(name, members_hint, subsets_hint, print_opts);

        Arc::from(Mutex::new(arc_set))
    }

    /// Creates a new ArcSet given a configuration.

    pub fn new_from_config(configuration: ArcSetConfig) -> ArcSet {
        let name       = configuration.name;
        let print_opts = configuration.print_opts;
        let title      = configuration.title;
        let id         = configuration.id;
        let next_id    = 1;
        let members    = Vec::with_capacity(configuration.members_hint);
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

        Arc::from(Mutex::new(subset))
    }

    /// Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Traverses the Rustics instances and subsets in the set invoking a
    /// user-supplied callback for each element.

    pub fn traverse(&mut self, traverser: &mut dyn ArcTraverser) {
        traverser.visit_set(self);

        for mutex in self.members.iter() {
            let mut member = mutex.lock().unwrap();

            traverser.visit_member(&mut *member);
        }

        for mutex in self.subsets.iter() {
            let mut subset = mutex.lock().unwrap();

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
            let member  = mutex.lock().unwrap();
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
            let subset  = mutex.lock().unwrap();
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

        for mutex in self.subsets.iter() {
            let mut subset  = mutex.lock().unwrap();
            let     title   = make_title(title, &subset.name());


            subset.set_title(&title);
        }

        for mutex in self.members.iter() {
            let mut member = mutex.lock().unwrap();
            let     title  = make_title(title, &member.name());

            member.set_title(&title);
        }
    }

    /// Does a recursive clear of all instances in the set and its
    /// entire subset hierarachy.

    pub fn clear(&mut self) {
        for mutex in self.subsets.iter() {
            let mut subset = mutex.lock().unwrap();

            subset.clear();
        }

        for mutex in self.members.iter() {
            let mut member = mutex.lock().unwrap();

            member.clear();
        }
    }

    /// Adds a Rustics member.  The user creates the statistics instance
    /// and passes it in an Arc.  This is a bit more manual than
    /// add_running_integer() and similar methods.

    pub fn add_member(&mut self, member: RusticsArc) {
        let mut stat  = member.lock().unwrap();
        let     title = make_title(&self.title, &stat.name());

        stat.set_title(&title);
        stat.set_id(self.next_id);
        self.next_id += 1;
        drop(stat);

        self.members.push(member);
    }

    /// Creates a RunningInteger instance and adds it to the set.

    pub fn add_running_integer(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let mut member  = RunningInteger::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a IntegerWindow instance and adds it to the set.

    pub fn add_integer_window(&mut self, name: &str, window_size: usize, units: UnitsOption)
            -> RusticsArc {
        let mut member = IntegerWindow::new(name, window_size, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningInteger as the base type and adds it to the set.

    pub fn add_integer_hier(&mut self, mut configuration: IntegerHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = IntegerHier::new_hier(configuration);
        let member = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a RunningTime instance and adds it to the set.  The user
    /// must provide a timer.  The timer can be used with the record_event
    /// method and is queried by the print routines to determine the hertz
    /// for the samples.

    pub fn add_running_time(&mut self, name: &str, timer: TimerBox) -> RusticsArc {
        let member  = RunningTime::new(name, timer, &self.print_opts);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a TimeWindow instance and adds it to the set.

    pub fn add_time_window(&mut self, name: &str, window_size: usize, timer: TimerBox)
            -> RusticsArc {
        let member  = TimeWindow::new(name, window_size, timer, &self.print_opts);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningTime as the base type and adds it to the set.

    pub fn add_time_hier(&mut self, mut configuration: TimeHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = TimeHier::new_hier(configuration);
        let member = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    // Creates a RunningFloat instance and adds it to the set.

    pub fn add_running_float(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let mut member = RunningFloat::new(name, &self.print_opts);

        if let Some(units) = units {
            member.set_units(units);
        }

        let member = Arc::from(Mutex::new(member));

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

        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a Hier using RunningFloat as the base type and adds it to the set.

    pub fn add_float_hier(&mut self, mut configuration: FloatHierConfig) -> RusticsArc {
        let print_opts =
            self.make_print_opts(&configuration.name, &configuration.print_opts);

        configuration.print_opts = print_opts;

        let member = FloatHier::new_hier(configuration);
        let member = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    /// Creates a Counter and adds it to the set.

    pub fn add_counter(&mut self, name: &str, units: UnitsOption) -> RusticsArc {
        let printer    = Some(self.printer.clone());
        let title      = None;
        let histo_opts = None;

        let print_opts = Some(PrintOpts { printer, title, units, histo_opts });

        let member  = Counter::new(name, &print_opts);
        let member  = Arc::from(Mutex::new(member));

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

    /// Removes a Rustics element from the set.

    pub fn remove_stat(&mut self, target_box: RusticsArc) -> bool {
        let mut found       = false;
        let mut i           = 0;
        let     target_stat = target_box.lock().unwrap();
        let     target_id   = target_stat.id();

        // We have to unlock the target_box or we'll hang in the loop.
        drop(target_stat);

        for mutex in self.members.iter() {
            let stat = mutex.lock().unwrap();
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

    pub fn add_subset(&mut self, name: &str, members_hint: usize, subsets_hint: usize)
            -> ArcSetBox {
        let name       = name.to_string();
        let title      = Some(make_title(&self.title, &name));
        let id         = self.next_id;
        let print_opts = self.print_opts.clone();

        let configuration =
            ArcSetConfig { name, members_hint, subsets_hint, title, id, print_opts };

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
            let subset = mutex.lock().unwrap();
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

    /// The following method is for internal use only.

    fn id(&self) -> usize {
        self.id
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::TestTimer;
    use crate::tests::ConverterTrait;
    use crate::tests::continuing_box;
    use crate::hier::Hier;
    use crate::Printer;
    use crate::time::Timer;
    use crate::time::DurationTimer;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;
    use std::time::Instant;

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

    //  Add statistics instances to a set.

    fn add_stats(parent: &Mutex<ArcSet>) {
        for i in 0..4 {
            let     lower         = -64;    // Just define the range for the test samples.
            let     upper         =  64;
            let     events_limit  = 2 * (upper - lower) as usize;

            let     parent        = &mut parent.lock().unwrap();
            let     subset_name   = format!("generated subset {}", i);
            let     subset        = parent.add_subset(&subset_name, 4, 4);
            let mut subset        = subset.lock().unwrap();

            let     window_name   = format!("generated window {}", i);
            let     running_name  = format!("generated running {}", i);
            let     window_mutex  = subset.add_integer_window(&window_name, events_limit, None);
            let     running_mutex = subset.add_running_integer(&running_name, None);

            let mut window        = window_mutex.lock().unwrap();
            let mut running       = running_mutex.lock().unwrap();

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

        let     set = ArcSet::new_box(&parent_name, 4, 4, &None);
        let mut set = set.lock().unwrap();

        //  Create timers for time statistics.

        let window_timer  = continuing_box();
        let running_timer = continuing_box();

        //  Now create the instances in our set.

        let window_size = 32;

        let window_mutex        = set.add_integer_window ("window",        window_size, None         );
        let running_mutex       = set.add_running_integer("running",       None                      );
        let time_window_mutex   = set.add_time_window    ("time window",   window_size, window_timer );
        let running_time_mutex  = set.add_running_time   ("running time",               running_timer);
        let float_window_mutex  = set.add_float_window  ("float window",  window_size, None          );
        let running_float_mutex = set.add_running_float ("running float", None                       );

        //  Lock the instances for manipulation.

        let mut window          = window_mutex       .lock().unwrap();
        let mut running         = running_mutex      .lock().unwrap();
        let mut time_window     = time_window_mutex  .lock().unwrap();
        let mut running_time    = running_time_mutex .lock().unwrap();
        let mut float_window    = float_window_mutex .lock().unwrap();
        let mut running_float   = running_float_mutex.lock().unwrap();

        //  Create some simple timers to be started manually.

        let     running_both  = TestTimer::new_box(test_hz);

        let     running_test  = ConverterTrait::as_test_timer(running_both.clone());
        let mut running_stat  = ConverterTrait::as_timer     (running_both.clone());

        let     window_both   = TestTimer::new_box(test_hz);

        let     window_test   = ConverterTrait::as_test_timer(window_both.clone());
        let mut window_stat   = ConverterTrait::as_timer     (window_both.clone());

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

        drop(subset);
        drop(subset_stat);
        drop(window);
        drop(running);
        drop(running_time);
        drop(time_window);
        drop(running_float);
        drop(float_window);

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

        let subset_1_name  = "subset 1";
        let subset_2_name  = "subset 2";
        let subset_1       = set.add_subset(subset_1_name, 4, 4);
        let subset_2       = set.add_subset(subset_2_name, 4, 4);

        let subset_1_impl  = subset_1.lock().unwrap();
        let subset_2_impl  = subset_2.lock().unwrap();

        assert!(subset_1_impl.title() == make_title(parent_name, &subset_1_name));
        assert!(subset_2_impl.title() == make_title(parent_name, &subset_2_name));

        drop(subset_1_impl);
        drop(subset_2_impl);

        add_stats(&subset_1);
        add_stats(&subset_2);

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

    //  Define a custom printer to check user-supplied printing.

    struct CustomPrinter {
    }

    impl Printer for CustomPrinter {
        fn print(&mut self, output: &str) {
            println!("CustomPrinter:  {}", output);
        }
    }

    fn new_hier() -> Hier {
        crate::hier::tests::make_hier(4, 8)
    }

    fn sample_usage() {
        // The last two parameters to new() are size hints, and need not be correct.
        // The same is true for add_subset.

        let      set     = ArcSet::new_box("parent set", 0, 1, &None);
        let mut  set     = set.lock().unwrap();
        let      subset  = set.add_subset("subset", 1, 0);
        let mut  subset  = subset.lock().unwrap();
        let      running = subset.add_running_integer("running", None);
        let mut  running = running.lock().unwrap();

        for i in 0..64 {
            running.record_i64(i);
        }

        //  Drop the locks before trying to print.

        drop(running);
        drop(subset);

        // Try a custom printer.

        let printer = Arc::new(Mutex::new(CustomPrinter { }));

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
        let member = Arc::from(Mutex::new(member));

        set.add_member(member);

        set.print_opts(Some(printer.clone()), None);

        // Try adding a hierarchical statistics instance.

        let hier_integer = new_hier();
        let member       = Arc::from(Mutex::new(hier_integer));

        set.add_member(member);

        set.print();
    }

    fn documentation() {
       // Create a set.  We're expecting 8 statistics instances but
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
   
       query_latency.lock().unwrap().record_time(local_timer.finish() as i64);
   
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

        let     integer_name   = "Integer Test";
        let     time_name      = "Time Test";
        let     float_name     = "Float Test";

        let     integer_config = make_integer_config(integer_name, auto_next, None);
        let     time_config    = make_time_config   (time_name,    auto_next, None);
        let     float_config   = make_float_config  (float_name,   auto_next, None);

        let     integer_hier   = set.add_integer_hier(integer_config);
        let     time_hier      = set.add_time_hier   (time_config   );
        let     float_hier     = set.add_float_hier  (float_config  );

        let mut integer_stat = integer_hier.lock().unwrap();
        let mut time_stat    = time_hier.lock   ().unwrap();
        let mut float_stat   = float_hier.lock  ().unwrap();

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

        let integer_generic = integer_stat.generic();
        let time_generic    = time_stat   .generic();
        let float_generic   = float_stat  .generic();

        let integer_hier    = integer_generic.downcast_ref::<Hier>().unwrap();
        let time_hier       = time_generic   .downcast_ref::<Hier>().unwrap();
        let float_hier      = float_generic  .downcast_ref::<Hier>().unwrap();

        assert!(integer_hier.event_count() == event_count);
        assert!(time_hier   .event_count() == event_count);
        assert!(float_hier  .event_count() == event_count);

        // Now drop the locks and print the set.

        drop(integer_stat);
        drop(time_stat   );
        drop(float_stat  );

        set.print();
    }

    #[test]
    pub fn run_tests() {
        simple_test();
        sample_usage();
        documentation();
        test_hier();
    }
}
