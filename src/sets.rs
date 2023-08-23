//
//  This code is available under the Berkeley 2-Clause license.  It is also available
//  as public domain source where permitted by law.
//

use std::sync::Mutex;
use std::sync::Arc;
use super::Rustics;
use super::RunningInteger;
use super::IntegerWindow;
use super::Printer;
use super::StdioPrinter;
use super::create_title;

pub type RusticsBox = Arc<Mutex<dyn Rustics>>;
pub type RusticsSetBox = Arc<Mutex<RusticsSet>>;
pub type PrinterBox = Arc<Mutex<dyn Printer>>;

// Define the trait for traversing a set and its hierarchy.

pub trait Traverser {
    fn visit_set(&mut self, set: &mut RusticsSet);
    fn visit_member(&mut self, member: &mut dyn Rustics);
}

pub struct RusticsSet {
    name:       String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsBox>,
    subsets:    Vec<RusticsSetBox>,
    printer:    PrinterBox,
}

impl RusticsSet {

    // Create a new set.

    pub fn new(name: &str, members: usize, subsets: usize) -> RusticsSet {
        let name = name.to_owned();
        let id = usize::max_value();
        let next_id = 0;
        let members = Vec::with_capacity(members);
        let subsets = Vec::with_capacity(subsets);
        let which = "stdout".to_string();
        let printer = Arc::new(Mutex::new(StdioPrinter { which }));

        RusticsSet { name, id, next_id, members, subsets, printer }
    }

    // Returns the name of the set.

    pub fn name(&self) -> String {
        self.name.clone()
    }

    // Traverses the statistics and subsets in the set invoking a
    // user-defined callback.

    pub fn traverse(&mut self, traverser: &mut dyn Traverser) {
        for mutex in self.members.iter() {
            let mut member = mutex.lock().unwrap();

            traverser.visit_member(&mut *member);
        }

        for mutex in self.subsets.iter() {
            let mut subset = mutex.lock().unwrap();

            traverser.visit_set(&mut subset);
        }
    }

    // Print the set and all its constituents (subsets and statistics).

    pub fn print(&self, title_prefix: &str) {
        let title = create_title(title_prefix, &self.name);

        for mutex in self.members.iter() {
            let member = mutex.lock().unwrap();

            member.print(&title);
        }

        for mutex in self.subsets.iter() {
            let subset = mutex.lock().unwrap();

            subset.print(&title);
        }
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

    // Create a RunningInteger statistics object and add it to the set.

    pub fn add_running_integer(&mut self, title: &str, printer: Option<PrinterBox>) -> &RusticsBox {
        self.members.push(Arc::from(Mutex::new(RunningInteger::new(title))));
        let result = self.members.last().unwrap();
        let mut stat = result.lock().unwrap();

        if let Some(printer) = printer {
            stat.set_printer(printer);
        } else {
            stat.set_printer(self.printer.clone());
        }

        stat.set_id(self.next_id);
        self.next_id += 1;
        result
    }

    // Create a IntegerWindow statistics object and add it to the set.

    pub fn add_integer_window(&mut self, window_size: usize, title: &str, printer: Option<PrinterBox>) -> &RusticsBox {
        self.members.push(Arc::from(Mutex::new(IntegerWindow::new(title, window_size))));
        let result = self.members.last().unwrap();
        let mut stat = result.lock().unwrap();

        if let Some(printer) = printer {
            stat.set_printer(printer);
        } else {
            stat.set_printer(self.printer.clone());
        }
        
        stat.set_id(self.next_id);
        self.next_id += 1;
        result
    }

    // Remove a statistic from the set.

    pub fn remove_stat(&mut self, target_box: &RusticsBox) -> bool {
        let mut found = false;
        let mut i = 0;
        let target_stat = target_box.lock().unwrap();
        let target_id = target_stat.id();

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

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> &RusticsSetBox {
        self.subsets.push(Arc::from(Mutex::new(RusticsSet::new(name, members, subsets))));
        let result = self.subsets.last().unwrap();
        let mut subset = result.lock().unwrap();
        subset.set_printer(self.printer.clone());
        subset.set_id(self.next_id);
        self.next_id += 1;

        result
    }

    // Remove a subset from the set.

    pub fn remove_subset(&mut self, target_box: &RusticsSetBox) -> bool {
        let mut found = false;
        let mut i = 0;
        let target_subset = target_box.lock().unwrap();
        let target_id = target_subset.id();

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

    pub fn set_printer(&mut self, printer: PrinterBox) {
        self.printer = printer;
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
mod tests {
    use super::*;

    fn add_stats(parent: &Mutex<RusticsSet>) {
        for _i in 0..4 {
            let lower = -64;
            let upper = 64;
            let parent = &mut parent.lock().unwrap();
            let mut subset = parent.add_subset("generated subset", 4, 4).lock().unwrap();
                
            let window_mutex = subset.add_integer_window(32, "generated subset window", None).clone();
            let running_mutex = subset.add_running_integer("generated subset running", None).clone();

            let mut window = window_mutex.lock().unwrap();
            let mut running = running_mutex.lock().unwrap();

            for i in lower..upper {
                window.record_i64(i);
                running.record_i64(i);
            }
        }
    }

    #[test]
    pub fn simple_test() {
        let lower = -32;
        let upper = 32;
        let mut set = RusticsSet::new("parent set", 4, 4);
            
        let window_mutex = set.add_integer_window(32, "parent window", None).clone();
        let running_mutex = set.add_running_integer("parent running", None).clone();

        let mut window = window_mutex.lock().unwrap();
        let mut running = running_mutex.lock().unwrap();

        for i in lower..upper {
            window.record_i64(i);
            running.record_i64(i);
        }

        drop(window);
        drop(running);
        set.print("Test Set");

        let printer = Arc::new(Mutex::new(TestPrinter { test_output: &"Sets Output" }));
        let mut traverser = TestTraverser::new(printer.clone());

        set.traverse(&mut traverser);
        set.set_printer(printer);

        let subset_1 = set.add_subset("subset 1", 4, 4).clone();
        let subset_2 = set.add_subset("subset 2", 4, 4).clone();

        add_stats(&subset_1);
        add_stats(&subset_2);

        println!("=========== Hierarchical Print");
        set.print("Test Hierarchy");

        // Remove a subset and check that it goes away.

        let found = set.remove_subset(&subset_1);
        assert!(found);

        let found = set.remove_subset(&subset_1);
        assert!(!found);

        // Remove two stats and check that they go away.
        //
        // First, do the remove operations.

        let found = set.remove_stat(&window_mutex);
        assert!(found);

        let found = set.remove_stat(&running_mutex);
        assert!(found);

        // Now check that the stats went away

        let found = set.remove_stat(&window_mutex);
        assert!(!found);

        let found = set.remove_stat(&running_mutex);
        assert!(!found);
    }

    struct TestPrinter {
        test_output: &'static str,
    }

    impl Printer for TestPrinter {
        fn print(&self, output: &str) {
            println!("{}:  {}", self.test_output, output);
        }
    }

    struct TestTraverser {
        printer:  Arc<Mutex<dyn Printer>>,
    }

    impl TestTraverser {
        pub fn new(printer: Arc<Mutex<dyn Printer>>) -> TestTraverser {
            TestTraverser { printer }
        }
    }

    impl Traverser for TestTraverser {
        fn visit_member(&mut self, member: &mut dyn Rustics) {
            member.set_printer(self.printer.clone());
        }

        fn visit_set(&mut self, set: &mut RusticsSet) {
            set.set_printer(self.printer.clone());
        }
    }
}
