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
use super::Printer;
use super::StdioPrinter;
use super::create_title;

pub type RusticsBox = Rc<RefCell<dyn Rustics>>;
pub type RusticsRcSetBox = Rc<RefCell<RusticsRcSet>>;
pub type PrinterBox = Arc<Mutex<dyn Printer>>;

// Define the trait for traversing a set and its hierarchy.

pub trait RcTraverser {
    fn visit_set(&mut self, set: &mut RusticsRcSet);
    fn visit_member(&mut self, member: &mut dyn Rustics);
}

pub struct RusticsRcSet {
    name:       String,
    id:         usize,
    next_id:    usize,
    members:    Vec<RusticsBox>,
    subsets:    Vec<RusticsRcSetBox>,
    printer:    PrinterBox,
}

impl RusticsRcSet {

    // Create a new set.
    //
    // The "members_hint" and "subsets_hint" parameters are hints as to the number
    // of elements to be expected.  "members_hint" refers to the number of Rustics
    // statistics in the set.  These hints can improve performance a bit.  They
    // might be especially useful in embedded environments.

    pub fn new(name: &str, members: usize, subsets: usize) -> RusticsRcSet {
        let name = name.to_owned();
        let id = usize::max_value();
        let next_id = 0;
        let members = Vec::with_capacity(members);
        let subsets = Vec::with_capacity(subsets);
        let which = "stdout".to_string();
        let printer = Arc::new(Mutex::new(StdioPrinter { which }));

        RusticsRcSet { name, id, next_id, members, subsets, printer }
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

    pub fn print(&self, title_prefix: &str) {
        let title = create_title(title_prefix, &self.name);

        for member in self.members.iter() {
            let member = (**member).borrow();
            member.print(&title);
        }

        for subset in self.subsets.iter() {
            let subset = (**subset).borrow_mut();
            subset.print(&title);
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

    pub fn add_running_integer(&mut self, title: &str, printer: Option<PrinterBox>) -> &RusticsBox {
        self.members.push(Rc::from(RefCell::new(RunningInteger::new(title))));
        let result = self.members.last().unwrap();
        let mut member = (**result).borrow_mut();

        if let Some(printer) = printer {
            member.set_printer(printer);
        } else {
            member.set_printer(self.printer.clone());
        }

        member.set_id(self.next_id);
        // self.members.last().unwrap().next_id += 1;
        result
    }

    // Create a IntegerWindow statistics object and add it to the set.

    pub fn add_integer_window(&mut self, window_size: usize, title: &str, printer: Option<PrinterBox>) -> &RusticsBox {
        self.members.push(Rc::from(RefCell::new(IntegerWindow::new(title, window_size))));
        let result = self.members.last().unwrap();
        let mut member = (**result).borrow_mut();

        if let Some(printer) = printer {
            member.set_printer(printer);
        } else {
            member.set_printer(self.printer.clone());
        }
        
        member.set_id(self.next_id);
        self.next_id += 1;
        result
    }

    // Remove a statistic from the set.

    pub fn remove_stat(&mut self, target: &RusticsBox) -> bool {
        let mut found = false;
        let mut i = 0;
        let member = (**target).borrow_mut();
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

    pub fn add_subset(&mut self, name: &str, members: usize, subsets: usize) -> &RusticsRcSetBox {
        self.subsets.push(Rc::from(RefCell::new(RusticsRcSet::new(name, members, subsets))));
        let result = self.subsets.last().unwrap();
        let mut subset = (**result).borrow_mut();
        subset.set_printer(self.printer.clone());
        subset.set_id(self.next_id);
        self.next_id += 1;

        result
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

    impl RcTraverser for TestTraverser {
        fn visit_member(&mut self, member: &mut dyn Rustics) {
            member.set_printer(self.printer.clone());
        }

        fn visit_set(&mut self, set: &mut RusticsRcSet) {
            set.set_printer(self.printer.clone());
        }
    }

    fn add_stats(parent: &mut RusticsRcSet) {
        // let parent_set = (*parent).borrow_mut();
        let parent_set = parent;

        for _i in 0..4 {
            let lower = -64;
            let upper = 64;
            let subset = parent_set.add_subset("generated subset", 4, 4);
            let mut subset = (**subset).borrow_mut();
                
            let window = subset.add_integer_window(32, "generated subset window", None).clone();
            let running = subset.add_running_integer("generated subset running", None).clone();

            let mut window = (*window).borrow_mut();
            let mut running = (*running).borrow_mut();

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
        let mut set = RusticsRcSet::new("parent set", 4, 4);
            
        let window = set.add_integer_window(32, "parent window", None).clone();
        let running = set.add_running_integer("parent running", None).clone();

        println!(" *** finished first add_* stats");

        let mut window_stat = (*window).borrow_mut();
        let mut running_stat = (*running).borrow_mut();

        for i in lower..upper {
            window_stat.record_i64(i);
            running_stat.record_i64(i);
        }

        drop(window_stat);
        drop(running_stat);

        println!(" *** finished first attempt at recording data");

        set.print("Test Set");

        let printer = Arc::new(Mutex::new(TestPrinter { test_output: &"Sets Output" }));
        let mut traverser = TestTraverser::new(printer.clone());

        set.traverse(&mut traverser);
        set.set_printer(printer);

        println!(" *** finished set_traver and set_printer");

        let subset_1 = set.add_subset("subset 1", 4, 4).clone();
        let subset_2 = set.add_subset("subset 2", 4, 4).clone();

        add_stats(&mut (*subset_1).borrow_mut());
        add_stats(&mut (*subset_2).borrow_mut());

        println!("=========== Hierarchical Print");
        set.print("Test Hierarchy");

        // Remove a subset and check that it goes away.

        let found = set.remove_subset(&subset_1);
        assert!(found);

        let found = set.remove_subset(&subset_1);
        assert!(!found);

        println!(" *** removed two subsets");

        // Remove two stats and check that they go away.
        //
        // First, do the remove operations.

        let found = set.remove_stat(&window);
        assert!(found);

        let found = set.remove_stat(&running);
        assert!(found);

        println!(" *** removed two stats");

        // Now check that the stats went away

        let found = set.remove_stat(&window);
        assert!(!found);

        let found = set.remove_stat(&running);
        assert!(!found);
    }
}
