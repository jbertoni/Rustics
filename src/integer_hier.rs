//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! ## Type
//!
//! * IntegerHier
//!     * This type implements hierarchical statistics using the
//!       RunningInteger type, q.v.
//!     * Each level uses a Window instance containing i RunningInteger
//!       instances, where i is configured per level.  See the window
//!       module documentation for more information on how the
//!       windows work.
//!     * Level 0 RunningInteger instances are used to collect data.
//!       Each instance collects n samples, where n is a configuration
//!       parameter.  After n samples are gathered, a new statistics
//!       instance is pushed into the window.
//!     * When k level 0 instances have been collected into the window,
//!       they are summed into one level 1 RunningInteger instance.  The
//!       value k is a configuration parameter.
//!     * A Rustics intance at level j is a sum of of i instance from
//!       level j - 1, where i is configured per level.
//!     * Each window retains RunningInteger instances that have
//!       already been summed, in case they are wanted for queries.
//!       The total window size is configured per level, and limits
//!       the number of retained members.
//!
//! ## Example
//!```
//!    // This example also is used in the Hier documentation, but some
//!    // of the assertions have been removed from that code.
//!
//!     use rustics::Rustics;
//!     use rustics::stdout_printer;
//!     use rustics::hier::Hier;
//!     use rustics::hier::HierDescriptor;
//!     use rustics::hier::HierDimension;
//!     use rustics::hier::HierIndex;
//!     use rustics::hier::HierSet;
//!     use rustics::integer_hier::IntegerHier;
//!     use rustics::integer_hier::IntegerHierConfig;
//!
//!     // Make a descriptor of the first level.  We have chosen to sum
//!     // 1000 level 0 RunningInteger instances into one level 1
//!     // RunningInteger instance.  This level is large, so we will keep
//!     // only 1000 level 0 instances in the window.
//!
//!     let dimension_0 = HierDimension::new(1000, 1000);
//!
//!     // At level 1, we want to sum 100 level 1 statistics into one
//!     // level 2 statistics instance.  This level is smaller, so let's
//!     // retain 200 RunningInteger instances here.
//!
//!     let dimension_1 = HierDimension::new(100, 200);
//!
//!     // Level two isn't summed, so the period isn't used.  Set the
//!     // value to one one event to keep the contructor happy.  Let's
//!     // pretend this level isn't used much, so retain only 100
//!     // instances in it.
//!
//!     let dimension_2 = HierDimension::new(1, 100);
//!
//!     //  Now create the Vec of the dimensions.
//!
//!     let dimensions =
//!         vec![ dimension_0, dimension_1, dimension_2 ];
//!
//!     // Now create the entire descriptor for the hier instance.  Let's
//!     // record 2000 events into each level 0 RunningInteger instance.
//!
//!     let auto_advance = Some(2000);
//!     let descriptor   = HierDescriptor::new(dimensions, auto_advance);
//!
//!     // Now specify some parameters used by Hier to do printing.  The
//!     // defaults for the title and printer are fine, so just pass None.
//!     // The title defaults to the name and output will go to stdout.
//!
//!     let name    = "test hierarchical integer".to_string();
//!     let title   = None;
//!     let printer = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         IntegerHierConfig { descriptor, name, title, printer };
//!
//!     // Now make the Hier instance and lock it.
//!
//!     let     integer_hier = IntegerHier::new_hier_box(configuration);
//!     let mut integer_hier = integer_hier.lock().unwrap();
//!
//!     // Now record some events with boring data.
//!
//!     let mut events   = 0;
//!     let auto_advance = auto_advance.unwrap();
//!
//!     for i in  0..auto_advance {
//!         events += 1;
//!         integer_hier.record_i64(i + 10);
//!     }
//!
//!     // We have just completed the first level 0 instance, but the
//!     // implementation creates the next instance only when it has data
//!     // to record, so there should be only one level zero instance,
//!     // and nothing at level 1 or level 2.
//!
//!     assert!(integer_hier.event_count() == events);
//!     assert!(integer_hier.count()       == events as u64);
//!     assert!(integer_hier.live_len(0)   == 1     );
//!     assert!(integer_hier.live_len(1)   == 0     );
//!     assert!(integer_hier.live_len(2)   == 0     );
//!
//!     // Now record some data to force the creation of the second level
//!     // 1 instance.
//!
//!     events += 1;
//!     integer_hier.record_i64(10);
//!
//!     // The new level 0 instance should have only one event recorded.
//!     // The Rustics implementatio for Hier returns the data in the
//!     // current level 0 instance, so check it.
//!
//!     assert!(integer_hier.event_count() == events);
//!     assert!(integer_hier.count()       == 1     );
//!     assert!(integer_hier.live_len(0)   == 2     );
//!     assert!(integer_hier.live_len(1)   == 0     );
//!     assert!(integer_hier.live_len(2)   == 0     );
//!
//!     let events_per_level_1 =
//!         auto_advance * dimension_0.period() as i64;
//!
//!     for i in events..events_per_level_1 {
//!         integer_hier.record_i64(i);
//!         events += 1;
//!     }
//!
//!     // Check the state again.  We need to record one more event to
//!     // cause the summation at level 0 into level 1.
//!
//!     let expected_live  = dimension_0.period();
//!     let expected_count = auto_advance as u64;
//!
//!     assert!(integer_hier.event_count() == events        );
//!     assert!(integer_hier.count()       == expected_count);
//!     assert!(integer_hier.live_len(0)   == expected_live );
//!     assert!(integer_hier.live_len(1)   == 0             );
//!     assert!(integer_hier.live_len(2)   == 0             );
//!
//!     integer_hier.record_i64(42);
//!     events += 1;
//!
//!     assert!(integer_hier.live_len(1)   == 1     );
//!     assert!(integer_hier.event_count() == events);
//!
//!     // Now print an element from the hierarchy.  In this case, we
//!     // will index into level 2, and print the third element of the
//!     // vector (index 2).  We use the set All to look at all the
//!     // elements in the window, not just the live elements.
//!
//!     let index = HierIndex::new(HierSet::All, 1, 2);
//!
//!     // The default printer and default title are fine for our
//!     // example, so pass None for the printer and title options.
//!
//!     integer_hier.print_index_opts(index, None, None);
//!```

//
// This module provides the interface between RunningInteger and the Hier
// code.
//

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use super::Rustics;
use super::Histogram;
use super::PrinterBox;
use super::PrinterOption;
use super::running_integer::RunningInteger;
use crate::running_integer::RunningExporter;

use crate::Hier;
use crate::HierBox;
use crate::HierDescriptor;
use crate::HierConfig;
use crate::HierGenerator;
use crate::HierMember;
use crate::HierExporter;
use crate::ExporterRc;
use crate::MemberRc;

// Provide for downcasting from a Hier member to a Rustics
// type or "dn Any" to get to the RunningInteger code.

impl HierMember for RunningInteger {
    fn to_rustics(&self) -> &dyn Rustics {
        self
    }

    fn to_rustics_mut(&mut self) -> &mut dyn Rustics {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn to_histogram(&self) -> &dyn Histogram {
        self as &dyn Histogram
    }
}

/// IntegerHier provides an interface from the Hier code to the
/// RunningInteger impl code that is not in methods.  Most users
/// should construct a Hier instance via functions like new_hier()
/// and new_hier_box() that do the type-specific initialization.

#[derive(Default)]
pub struct IntegerHier {
}

/// IntegerHierConfig is used to pass the constructor parameters
/// for a Hier instance that uses RunningInteger statistics.

#[derive(Clone)]
pub struct IntegerHierConfig {
    pub descriptor:  HierDescriptor,
    pub name:        String,
    pub title:       Option<String>,
    pub printer:     PrinterOption,
}

impl IntegerHier {
    pub fn new_raw() -> IntegerHier  {
        IntegerHier { }
    }

    /// new_hier() creates a new Hier instance from the given
    /// configuration.  This function does the grunt work specific
    /// to the RunningInteger type.

    pub fn new_hier(configuration: IntegerHierConfig) -> Hier {
        let generator  = IntegerHier::new_raw();
        let generator  = Rc::from(RefCell::new(generator));
        let class      = "integer".to_string();

        let descriptor = configuration.descriptor;
        let name       = configuration.name;
        let title      = configuration.title;
        let printer    = configuration.printer;

        let config = HierConfig { descriptor, generator, name, title, class, printer };

        Hier::new(config)
    }

    /// new_hier_box() uses new_hier() to create a Hier instance and
    /// returns it as an Arc<Mutex<Hier>> for multi-threaded
    /// use.

    pub fn new_hier_box(configuration: IntegerHierConfig) -> HierBox {
        let hier = IntegerHier::new_hier(configuration);

        Arc::from(Mutex::new(hier))
    }
}

// These are the methods that the Hier instance needs implemented
// for a given statistic type that are not specific to a member
// of that type.  It's thus the bridge between "impl RunningInteger"
// and the Hier code.

impl HierGenerator for IntegerHier {
    fn make_member(&self, name: &str, printer: PrinterBox) -> MemberRc {
        let member = RunningInteger::new(name, Some(printer));

        Rc::from(RefCell::new(member))
    }

    // Make a member from a complete list of exported statistics.

    fn make_from_exporter(&self, name: &str, printer: PrinterBox, exporter: ExporterRc) -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<RunningExporter>().unwrap();
        let     member          = exporter_impl.make_member(name, printer);

        Rc::from(RefCell::new(member))
    }

    fn make_exporter(&self) -> ExporterRc {
        let exporter = RunningExporter::new();

        Rc::from(RefCell::new(exporter))
    }

    // Push another statistic onto the export list.  We will sum all of
    // them at some point.

    fn push(&self, exporter: &mut dyn HierExporter, member_rc: MemberRc) {
        let     exporter_any    = exporter.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<RunningExporter>().unwrap();

        let     member_borrow   = member_rc.borrow();
        let     member_any      = member_borrow.as_any();
        let     member_impl     = member_any.downcast_ref::<RunningInteger>().unwrap();

        exporter_impl.push(member_impl.export());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdout_printer;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;

    fn make_test_hier(auto_next: i64) -> Hier {
        let     levels         = 4;
        let     level_0_period = 8;
        let     dimension      = HierDimension::new(level_0_period, 3 * level_0_period);
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

        let descriptor    = HierDescriptor::new(dimensions, Some(auto_next));
        let generator     = IntegerHier::new_raw();
        let generator     = Rc::from(RefCell::new(generator));
        let class         = "integer".to_string();
        let name          = "test hier".to_string();
        let title         = None;
        let printer       = Some(stdout_printer());

        let configuration = HierConfig { descriptor, generator, class, name, title, printer };

        Hier::new(configuration)
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_simple_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     generator    = IntegerHier::new_raw();
        let     printer      = stdout_printer();
        let     member_rc    = generator.make_member("test member", printer);
        let     member_clone = member_rc.clone();
        let mut member       = member_clone.borrow_mut();
        let     value        = 42;

        member.to_rustics_mut().record_i64(value);

        assert!(member.to_rustics().count() == 1);
        assert!(member.to_rustics().mean()  == value as f64);

        // Drop the lock on the member.

        drop(member);

        // Now try try making an exporter and check basic sanity of as_any_mut.

        let exporter_rc     = generator.make_exporter();
        let exporter_clone  = exporter_rc.clone();

        // Push the member's numbers onto the exporter.

        generator.push(&mut *exporter_clone.borrow_mut(), member_rc);

        let name    = "member export";
        let printer = stdout_printer();

        let new_member_rc = generator.make_from_exporter(name, printer, exporter_rc);

        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);

        // Now make an actual hier instance.

        let     auto_next = 200;
        let mut hier      = make_test_hier(auto_next);
        let mut events    = 0;

        for i in 1..auto_next / 2 {
            hier.record_i64(i);

            events += 1;
        }

        let float    = events as f64;
        let mean     = (float * (float + 1.0) / 2.0) / float;

        assert!(hier.mean() == mean);
        assert!(hier.event_count() == events);
        hier.print();
    }

    #[test]
    fn run_tests() {
        test_simple_running_generator();
    }
}
