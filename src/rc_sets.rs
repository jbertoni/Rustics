//
//  This code is available under the Berkeley 2-Clause license.  It is also available
//  as public domain source where permitted by law.
//

use std::sync::Arc;
use std::sync::Mutex;
use std::rc::Rc;
use std::cell::RefCell;
use super::Rustics;
use super::RunningInteger;
use super::IntegerWindow;
use super::RunningTime;
use super::TimeWindow;
use super::TimerBox;
use super::Printer;
use super::create_title;

pub type RusticsRc = Rc<RefCell<dyn Rustics>>;
pub type RusticsRcSetBox = Rc<RefCell<RusticsRcSet>>;
pub type PrinterBox = Arc<Mutex<dyn Printer>>;

// Define the trait for traversing a set and its hierarchy.

pub trait RcTraverser {
    fn visit_set(&mut self, set: &mut RusticsRcSet);
    fn visit_member(&mut self, member: &mut dyn Rustics);
}

pub struct RusticsRcSet {
    name:       String,
    title:      String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsRc>,
    subsets:    Vec<RusticsRcSetBox>,
}

impl RusticsRcSet {

    // Create a new set.
    //
    // The "members_hint" and "subsets_hint" parameters are hints as to the number
    // of elements to be expected.  "members_hint" refers to the number of Rustics
    // statistics in the set.  These hints can improve performance a bit.

    pub fn new(name_in: &str, members: usize, subsets: usize) -> RusticsRcSet {
        let name = String::from(name_in);
        let title = String::from(name_in);
        let id = usize::MAX;
        let next_id = 0;
        let members = Vec::with_capacity(members);
        let subsets = Vec::with_capacity(subsets);

        RusticsRcSet { name, title, id, next_id, members, subsets }
    }

    // Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Traverses the statistics and subsets in the set invoking a
    // user-defined callback.

    pub fn traverse(&mut self, traverser: &mut dyn RcTraverser) {
        for member in self.members.iter() {
            traverser.visit_member(&mut *((**member).borrow_mut()));
        }

        for subset in self.subsets.iter() {
            traverser.visit_set(&mut (**subset).borrow_mut());
        }
    }

    // Print the set and all its constituents (subsets and statistics).

    pub fn print(&self, printer: Option<PrinterBox>) {
        for member in self.members.iter() {
            let member = (**member).borrow();
            member.print(printer.clone());
        }

        for subset in self.subsets.iter() {
            let subset = (**subset).borrow_mut();
            subset.print(printer.clone());
        }
    }

    // Do a recursive clear of all statistics in the set and its
    // entire subset hierarachy.

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

    // Create a RunningInteger statistics object and add it to the set.

    pub fn add_running_integer(&mut self, title: &str) -> RusticsRc {
        self.members.push(Rc::from(RefCell::new(RunningInteger::new(title))));
        self.common_add()
    }

    // Create a IntegerWindow statistics object and add it to the set.

    pub fn add_integer_window(&mut self, window_size: usize, title: &str) -> RusticsRc {
        self.members.push(Rc::from(RefCell::new(IntegerWindow::new(title, window_size))));
        self.common_add()
    }

    pub fn add_running_time(&mut self, title: &str, timer: TimerBox) -> RusticsRc {
        self.members.push(Rc::from(RefCell::new(RunningTime::new(title, timer))));
        self.common_add()
    }

    pub fn add_time_window(&mut self, title: &str, window_size: usize, timer: TimerBox) -> RusticsRc {
        self.members.push(Rc::from(RefCell::new(TimeWindow::new(title, window_size, timer))));
        self.common_add()
    }

    fn common_add(&mut self) -> RusticsRc {
        let last = self.members.last().unwrap();
        let mut member = (**last).borrow_mut();

        let title = create_title(&self.title, &member.name());
        member.set_title(&title);
        member.set_id(self.next_id);
        self.next_id += 1;
        last.clone()
    }

    // Remove a statistic from the set.

    pub fn remove_stat(&mut self, target: RusticsRc) -> bool {
        let mut found = false;
        let mut i = 0;
        let member = (*target).borrow_mut();
        let target_id = member.id();
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

    // Create a new subset and add it to the set.

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> RusticsRcSetBox {
        self.subsets.push(Rc::from(RefCell::new(RusticsRcSet::new(name, members, subsets))));
        let last = self.subsets.last().unwrap();
        let mut subset = (**last).borrow_mut();
        let title = create_title(&self.title, name);
        subset.set_title(&title);
        subset.set_id(self.next_id);
        self.next_id += 1;

        last.clone()
    }

    // Remove a subset from the set.

    pub fn remove_subset(&mut self, target: &RusticsRcSetBox) -> bool {
        let mut found = false;
        let mut i = 0;
        let subset = (**target).borrow_mut();
        let target_id = subset.id();

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

    // The following functions are for internal use only.
    #[cfg(test)]
    fn title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, title: &str) {
        self.title = String::from(title);
    }

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
    use crate::time::Timer;

    struct TestTraverser {
    }

    impl TestTraverser {
        pub fn new() -> TestTraverser {
            TestTraverser { }
        }
    }

    impl RcTraverser for TestTraverser {
        fn visit_member(&mut self, _member: &mut dyn Rustics) {
        }

        fn visit_set(&mut self, _set: &mut RusticsRcSet) {
        }
    }

    fn add_stats(parent: &mut RusticsRcSet) {
        let parent_set = parent;

        for _i in 0..4 {
            let lower = -64;
            let upper = 64;
            let subset = parent_set.add_subset("generated subset", 4, 4);
            let mut subset = (*subset).borrow_mut();
                
            let window = subset.add_integer_window(32, "generated subset window");
            let running = subset.add_running_integer("generated subset running");

            let mut window = (*window).borrow_mut();
            let mut running = (*running).borrow_mut();

            for i in lower..upper {
                window.record_i64(i);
                running.record_i64(i);
            }
        }
    }

    // Define a simple timer for testing that just counts up by 1000 ticks
    // for each event interval.

    struct ContinuingTimer {
        time: u128,
        hz:   u128,
    }

    impl ContinuingTimer {
        pub fn new(hz: u128) -> ContinuingTimer {
            let time = 0;

            ContinuingTimer { time, hz }
        }
    }

    impl Timer for ContinuingTimer {
        fn start(&mut self) {
            self.time = 0;
        }

        fn finish(&mut self) -> u128 {
            self.time += 1000;
            self.time
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    pub fn simple_test() {
        let lower = -32;
        let upper = 32;
        let mut set = RusticsRcSet::new("parent set", 4, 4);

        let window = set.add_integer_window(32, "window");
        let running = set.add_running_integer("running");

        println!(" *** finished first add_* stats");

        let window_timer:  TimerBox = Rc::from(RefCell::new(ContinuingTimer::new(1_000_000_000)));
        let running_timer: TimerBox = Rc::from(RefCell::new(ContinuingTimer::new(1_000_000_000)));
            
        let time_window = set.add_time_window("time window", 32, window_timer);
        let running_time = set.add_running_time("running time", running_timer);

        println!(" *** finished first timer add_* stats");

        let mut window_stat = (*window).borrow_mut();
        let mut running_stat = (*running).borrow_mut();

        let mut time_window_stat = (*time_window).borrow_mut();
        let mut running_time_stat = (*running_time).borrow_mut();

        for i in lower..upper {
            window_stat.record_i64(i);
            running_stat.record_i64(i);

            time_window_stat.record_event();
            running_time_stat.record_event();
        }

        let set_title = set.title();
        assert!(set_title == "parent set");
        assert!(running_time_stat.title() == create_title(&"parent set", &"running time"));
        assert!(time_window_stat.title() == create_title(&"parent set", &"time window"));
        assert!(running_stat.title() == create_title(&"parent set", &"running"));
        assert!(window_stat.title() == create_title(&"parent set", &"window"));

        let subset = set.add_subset("subset", 0, 0);
        let mut subset = (*subset).borrow_mut();
        let subset_title = subset.title();

        let subset_stat = subset.add_running_integer("subset stat");
        let subset_stat = (*subset_stat).borrow_mut();

        assert!(subset_title == create_title(&set_title, "subset"));
        assert!(subset_stat.title() == create_title(&subset_title, &"subset stat"));

        drop(window_stat);
        drop(running_stat);
        drop(subset_stat);
        drop(subset);

        drop(time_window_stat);
        drop(running_time_stat);

        println!(" *** finished first attempt at recording data");

        set.print(None);

        let mut traverser = TestTraverser::new();

        set.traverse(&mut traverser);

        println!(" *** finished set_traver");

        let subset_1 = set.add_subset("subset 1", 4, 4);
        let subset_2 = set.add_subset("subset 2", 4, 4);

        add_stats(&mut (*subset_1).borrow_mut());
        add_stats(&mut (*subset_2).borrow_mut());

        println!("=========== Hierarchical Print");
        set.print(None);

        // Remove a subset and check that it goes away.

        let found = set.remove_subset(&subset_1);
        assert!(found);

        let found = set.remove_subset(&subset_1);
        assert!(!found);

        println!(" *** removed two subsets");

        // Remove two stats and check that they go away.
        //
        // First, do the remove operations.  We must clone the
        // rc objects since the call moves them.

        let found = set.remove_stat(window.clone());
        assert!(found);

        let found = set.remove_stat(running.clone());
        assert!(found);

        println!(" *** removed two stats");

        // Now check that the stats went away

        let found = set.remove_stat(window);
        assert!(!found);

        let found = set.remove_stat(running);
        assert!(!found);
    }

    fn sample_usage() {
        // The last two parameters to new() and add_subset are size hints.
        // They are only hints.

        let mut set     = RusticsRcSet::new("sample usage parent", 0, 0);

        // Add a subset.

        let     subset  = set.add_subset("subset", 0, 0);
        let mut subset  = (*subset).borrow_mut();

        // Create a running integer statistic.

        let     running = subset.add_running_integer("generated subset running");
        let mut running = (*running).borrow_mut();

        // Now try a timer window.

        let     window_timer = Rc::from(RefCell::new(ContinuingTimer::new(1_000_000_000)));
        let     time_window  = set.add_time_window("time window", 32, window_timer);
        let mut time_window  = (*time_window).borrow_mut();
        
        let mut timer: TimerBox = Rc::from(RefCell::new(ContinuingTimer::new(1_000_000_000)));

        (*timer).borrow_mut().start();

        println!(" *** finished first timer add_* stats");

        for i in -32..64 {
            running.record_i64(i);
            time_window.record_event();
            time_window.record_interval(&mut timer);
        }

        // Drop the locks before trying to print.

        drop(running);
        drop(subset);
        drop(time_window);

        set.print(None);
    }

    #[test]
    pub fn run_tests() {
        simple_test();
        sample_usage();
    }
}
