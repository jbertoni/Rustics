//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

use std::sync::Mutex;
use std::sync::Arc;
use super::Rustics;
use super::running_integer::RunningInteger;
use super::running_time::RunningTime;
use super::integer_window::IntegerWindow;
use super::time_window::TimeWindow;
use super::stdout_printer;
use super::counter::Counter;
use super::TimerBox;
use super::PrinterBox;
use super::PrinterOption;
use super::create_title;

pub type RusticsArc = Arc<Mutex<dyn Rustics>>;
pub type ArcSetBox  = Arc<Mutex<ArcSet>>;

// Define the trait for traversing a set and its hierarchy.

pub trait ArcTraverser {
    fn visit_set(&mut self, set: &mut ArcSet);
    fn visit_member(&mut self, member: &mut dyn Rustics);
}

// Define the set type.  A set can contain statistics and
// subsets of type ArcSet.

pub struct ArcSet {
    name:       String,
    title:      String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsArc>,
    subsets:    Vec<ArcSetBox>,
    printer:    PrinterBox,
}

impl ArcSet {

    // Create a new set.
    //
    // The "members_hint" and "subsets_hint" parameters are hints as to the number
    // of elements to be expected.  "members_hint" refers to the number of Rustics
    // statistics in the set.  These hints can improve performance a bit.  They
    // might be especially useful in embedded environments.

    pub fn new(name_in: &str, printer: PrinterOption, members_hint: usize, subsets_hint: usize)
            -> ArcSet {
        let name    = String::from(name_in);
        let title   = String::from(name_in);
        let id      = usize::MAX;
        let next_id = 1;
        let members = Vec::with_capacity(members_hint);
        let subsets = Vec::with_capacity(subsets_hint);

        let printer =
            if let Some(printer) = printer {
                printer
            } else {
                stdout_printer()
            };

        ArcSet { name, title, id, next_id, members, subsets, printer }
    }

    // Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Traverses the statistics and subsets in the set invoking a
    // user-defined callback.

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

    // Print the set and all its constituents (subsets and statistics).

    pub fn print(&self) {
        self.print_opts(None, None);
    }

    pub fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        for mutex in self.members.iter() {
            let member = mutex.lock().unwrap();

            member.print_opts(printer.clone(), title);
        }

        for mutex in self.subsets.iter() {
            let subset = mutex.lock().unwrap();

            subset.print_opts(printer.clone(), title);
        }
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = String::from(title);
    }

    // Do a recursive clear of all statistics in the set and its
    // entire subset hierarachy.

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

    // Add a member statistic.  The user creates the statistics object
    // and passes it in an Arc.

    pub fn add_member(&mut self, member: RusticsArc) {
        let mut stat  = member.lock().unwrap();
        let     title = create_title(&self.title, &stat.name());

        stat.set_title(&title);
        stat.set_id(self.next_id);
        self.next_id += 1;
        drop(stat);

        self.members.push(member);
    }

    // Create a RunningInteger statistics object and add it to the set.

    pub fn add_running_integer(&mut self, name: &str) -> RusticsArc {
        let printer = Some(self.printer.clone());
        let member  = RunningInteger::new(name, printer);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    // Create a IntegerWindow statistics object and add it to the set.

    pub fn add_integer_window(&mut self, window_size: usize, name: &str) -> RusticsArc {
        let printer = Some(self.printer.clone());
        let member  = IntegerWindow::new(name, window_size, printer);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    pub fn add_running_time(&mut self, name: &str, timer: TimerBox) -> RusticsArc {
        let printer = Some(self.printer.clone());
        let member  = RunningTime::new(name, timer, printer);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    pub fn add_time_window(&mut self, name: &str, window_size: usize, timer: TimerBox) -> RusticsArc {
        let printer = Some(self.printer.clone());
        let member  = TimeWindow::new(name, window_size, timer, printer);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    pub fn add_counter(&mut self, name: &str) -> RusticsArc {
        let printer = Some(self.printer.clone());
        let member  = Counter::new(name, printer);
        let member  = Arc::from(Mutex::new(member));

        self.add_member(member.clone());
        member
    }

    // Remove a statistic from the set.

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

    // Create a new subset and add it to the set.

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> ArcSetBox {
        let printer = Some(self.printer.clone());
        let subset  = ArcSet::new(name, printer, members, subsets);
        let subset  = Arc::from(Mutex::new(subset));

        self.subsets.push(subset);

        let     last   = self.subsets.last().unwrap();
        let mut subset = last.lock().unwrap();
        let     title  = create_title(&self.title, name);

        subset.set_title(&title);
        subset.set_id(self.next_id);
        self.next_id += 1;

        last.clone()
    }

    // Remove a subset from the set.

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

    // The following functions are for internal use only.

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::continuing_box;
    use crate::hier::Hier;
    use crate::Printer;
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::time::Timer;

    //  Add statistics to a set.

    fn add_stats(parent: &Mutex<ArcSet>) {
        for i in 0..4 {
            let lower             = -64;    // Just define the range for the test samples.
            let upper             = 64;

            let     parent        = &mut parent.lock().unwrap();
            let     name          = format!("generated subset {}", i);
            let     subset        = parent.add_subset(&name, 4, 4);
            let mut subset        = subset.lock().unwrap();

            let     window_name   = format!("generated window {}", i);
            let     running_name  = format!("generated running {}", i);
            let     window_mutex  = subset.add_integer_window(32, &window_name);
            let     running_mutex = subset.add_running_integer(&running_name);

            let mut window         = window_mutex.lock().unwrap();
            let mut running        = running_mutex.lock().unwrap();

            println!(" *** made arc \"{}\"", subset.title());
            println!(" *** made arc \"{}\"", window.title());
            println!(" *** made arc \"{}\"", running.title());

            for i in lower..upper {
                window.record_i64(i);
                running.record_i64(i);
            }
        }
    }

    //  Implement a timer time that lets us control the interval values.
    //  This approach lets us test various properties of the underlying
    //  statistics.  This timer is "one-shot":  it must be reinitialized
    //  for each timer operation.

    static global_next: Mutex<u128> = Mutex::new(0 as u128);

    fn get_global_next() -> u128 {
        *(global_next.lock().unwrap())
    }

    fn set_global_next(value: u128) {
        *(global_next.lock().unwrap()) = value;
    }

    struct TestTimer {
        start: u128,
        hz: u128,
    }

    impl TestTimer {
        fn new(hz: u128) -> TestTimer {
            let start = 0;

            TestTimer { start, hz }
        }
    }

    impl Timer for TestTimer {
        fn start(&mut self) {
            assert!(get_global_next() > 0);
            self.start = get_global_next();
        }

        fn finish(&mut self) -> u128 {
            assert!(self.start > 0);
            assert!(get_global_next() >= self.start);
            let elapsed_time = get_global_next() - self.start;
            self.start = 0;
            set_global_next(0);
            elapsed_time
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    //  Given a timer, set the start time and set the elapsed
    //  time that will be reported.

    fn setup_elapsed_time(timer: &mut TimerBox, ticks: i64) {
        assert!(ticks >= 0);
        let mut timer = (**timer).borrow_mut();
        set_global_next(1);
        timer.start();
        set_global_next(ticks as u128 + 1);
    }

    pub fn simple_test() {
        let lower   = -32;
        let upper   = 32;
        let test_hz = 1_000_000_000;

        //  Create the parent set for our test statistics.

        let mut set = ArcSet::new("parent set", None, 4, 4);

        //  Create timers for time statistics.

        let window_timer:  TimerBox = continuing_box();
        let running_timer: TimerBox = continuing_box();

        //  Now create the statistics in our set.

        let window_mutex       = set.add_integer_window(32, "window");
        let running_mutex      = set.add_running_integer("running");
        let time_window_mutex  = set.add_time_window("time window", 32, window_timer);
        let running_time_mutex = set.add_running_time("running time", running_timer);

        //  Lock the statistics for manipulation.

        let mut window         = window_mutex.lock().unwrap();
        let mut running        = running_mutex.lock().unwrap();
        let mut time_window    = time_window_mutex.lock().unwrap();
        let mut running_time   = running_time_mutex.lock().unwrap();

        //  Create some simple timers to be started manually.

        let mut running_interval: TimerBox = Rc::from(RefCell::new(TestTimer::new(test_hz)));
        let mut window_interval:  TimerBox = Rc::from(RefCell::new(TestTimer::new(test_hz)));

        //  Now record some data in all the statistics.

        for i in lower..upper {
            window.record_i64(i);
            running.record_i64(i);

            running_time.record_event();
            time_window.record_event();

            setup_elapsed_time(&mut running_interval, 10 + i.abs() * 10);
            running_time.record_interval(&mut running_interval);

            setup_elapsed_time(&mut window_interval, 1000 + i.abs() * 10000);
            time_window.record_interval(&mut window_interval);
        }

        //  Make sure the titles are being created properly.

        let set_title = set.title();

        assert!(set_title == "parent set");
        assert!(running_time.title() == create_title(&"parent set", &"running time"));
        assert!(time_window.title() == create_title(&"parent set", &"time window"));
        assert!(running.title() == create_title(&"parent set", &"running"));
        assert!(window.title() == create_title(&"parent set", &"window"));

        //  Create a subset to check titles in a subtree.

        let     subset = set.add_subset("subset", 0, 0);
        let mut subset = subset.lock().unwrap();
        let subset_stat_mutex = subset.add_running_integer("subset stat");
        let subset_stat   = subset_stat_mutex.lock().unwrap();

        assert!(subset.title() == create_title(&set_title, "subset"));
        assert!(subset_stat.title() == create_title(&subset.title(), &"subset stat"));

        //  Drop all the locks.

        drop(subset);
        drop(subset_stat);
        drop(window);
        drop(running);
        drop(running_time);
        drop(time_window);

        //  Make sure that print completes.

        set.print();

        //  Do a test of the traverser.

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** arc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 5);
        assert!(traverser.sets == 2);

        //  Now test removing statistics.

        let subset_1 = set.add_subset("subset 1", 4, 4);
        let subset_2 = set.add_subset("subset 2", 4, 4);

        add_stats(&subset_1);
        add_stats(&subset_2);

        // Before testing remove operations, traverse again...

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);
        println!(" *** arc members {}, sets {}", traverser.members, traverser.sets);

        assert!(traverser.members == 21);
        assert!(traverser.sets == 12);

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
        fn print(&self, output: &str) {
            println!("CustomPrinter:  {}", output);
        }
    }

    fn new_hier() -> Hier {
        crate::hier::tests::make_hier(4, 8)
    }

    fn sample_usage() {
        // The last two parameters to new() are size hints, and need not be correct.
        // The same is true for add_subset.

        let mut  set     = ArcSet::new("parent set", None, 0, 1);
        let      subset  = set.add_subset("subset", 1, 0);
        let mut  subset  = subset.lock().unwrap();
        let      running = subset.add_running_integer("running");
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

        let     counter_arc = set.add_counter("test counter");
        let mut counter     = counter_arc.lock().unwrap();
        let     limit       = 20;

        for _i in 1..limit + 1 {
            counter.record_event();    // increment by 1
            counter.record_i64(1);     // increment by 1
        }

        //  Check the counter value.

        assert!(counter.count() == 2 * limit as u64);

        //  Drop the lock before printing.

        drop(counter);

        //  print should still work.

        let member = Arc::from(Mutex::new(RunningInteger::new("added as member", None),));
        set.add_member(member);

        set.print_opts(Some(printer.clone()), None);

        // Try adding a hierarchical statistic.

        let hier_integer = new_hier();
        let member       = Arc::from(Mutex::new(hier_integer));

        set.add_member(member);

        set.print();
    }

    #[test]
    pub fn run_tests() {
        simple_test();
        sample_usage();
    }

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
}
