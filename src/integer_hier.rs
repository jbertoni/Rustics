//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//! ## Type
//!
//! * IntegerHier
//!     * This type implements hierarchical statistics using the
//!       RunningInteger type, q.v.
//!
//!     * Each level uses a Window instance containing a configurable
//!       number of RunningInteger instances.  See the window module
//!       documentation for more information on how the windows work.
//!
//!     * Level 0 RunningInteger instances are used to collect data.
//!       Each instance collects a configurable number of samples.
//!       After that number of samples are gathered, a new
//!       RunningInteger instance is pushed into the window.
//!
//!     * When a configurable number of level 0 instances have been
//!       collected into the window, they are summed into one level 1
//!       RunningInteger instance.
//!
//!     * In general, a Rustics intance at level j is a sum of of i
//!       instances from level j - 1, where i is configured per level.
//!
//!     * Each window retains RunningInteger instances that have
//!       already been summed, in case they are wanted for queries.
//!       The total window size is configured per level, and limits
//!       the number of retained RunningInteger instances.
//!
//! ## Example
//!```
//!    // This example also is used in the Hier documentation.
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
//!     // At level 1, we want to sum 100 level 1 instances into one level
//!     // 2 instance.  This level is smaller, so let's retain 200
//!     // RunningInteger instances here.
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
//!     // Don't configure a window for this example.
//!
//!     let name        = "test hierarchical integer".to_string();
//!     let print_opts  = None;
//!     let window_size = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         IntegerHierConfig { descriptor, name, window_size, print_opts };
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
//!     // Print our data.
//!
//!     integer_hier.print();
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
use super::PrintOption;
use super::running_integer::RunningInteger;
use crate::running_integer::IntegerExporter;
use super::integer_window::IntegerWindow;

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
///
/// See the module comments for a sample program.

#[derive(Default)]
pub struct IntegerHier {
}

/// IntegerHierConfig is used to pass the constructor parameters
/// for a Hier instance that uses the RunningInteger type for
/// recording and combining data.  The window_size parameter can
/// be set to cause the Hier instance to maintain a window of the
/// last n events to be used for its Rustics reporting.

pub struct IntegerHierConfig {
    pub descriptor:  HierDescriptor,
    pub name:        String,
    pub print_opts:  PrintOption,
    pub window_size: Option<usize>,
}

impl IntegerHier {
    pub fn new() -> IntegerHier  {
        IntegerHier { }
    }

    /// new_hier() creates a new Hier instance from the given
    /// configuration.  This function does the grunt work specific
    /// to the RunningInteger type.

    pub fn new_hier(configuration: IntegerHierConfig) -> Hier {
        let generator    = IntegerHier::new();
        let generator    = Rc::from(RefCell::new(generator));
        let class        = "integer".to_string();

        let descriptor   = configuration.descriptor;
        let name         = configuration.name;
        let print_opts   = configuration.print_opts;
        let window_size  = configuration.window_size;

        let config = HierConfig { descriptor, generator, name, window_size, class, print_opts };

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
// for a given statistics type that are not specific to an instance
// of that type.  It's thus the bridge between "impl RunningInteger"
// and the Hier code.

impl HierGenerator for IntegerHier {
    fn make_member(&self, name: &str, print_opts: &PrintOption) -> MemberRc {
        let member = RunningInteger::new(name, print_opts);

        Rc::from(RefCell::new(member))
    }

    fn make_window(&self, name: &str, window_size: usize, print_opts: &PrintOption)
            -> Box<dyn Rustics> {
        let window = IntegerWindow::new(name, window_size, print_opts);

        Box::new(window)
    }

    // Make a member from a complete list of exported statistics.

    fn make_from_exporter(&self, name: &str, print_opts: &PrintOption, exporter: ExporterRc)
            -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<IntegerExporter>().unwrap();
        let     member          = exporter_impl.make_member(name, print_opts);

        Rc::from(RefCell::new(member))
    }

    fn make_exporter(&self) -> ExporterRc {
        let exporter = IntegerExporter::new();

        Rc::from(RefCell::new(exporter))
    }

    // Push another instance onto the export list.  We will sum all of
    // them at some point.

    fn push(&self, exporter: &mut dyn HierExporter, member_rc: MemberRc) {
        let     exporter_any    = exporter.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<IntegerExporter>().unwrap();

        let     member_borrow   = member_rc.borrow();
        let     member_any      = member_borrow.as_any();
        let     member_impl     = member_any.downcast_ref::<RunningInteger>().unwrap();

        exporter_impl.push(member_impl.export_data());
    }

    fn hz(&self) -> u128 {
        panic!("IntegerHier::hz:  not supported");
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;

    fn level_0_period() -> usize {
        8
    }

    fn level_0_retain() -> usize {
        3 * level_0_period()
    }

    pub fn make_test_hier(auto_next: i64, window_size: Option<usize>) -> Hier {
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

        let descriptor    = HierDescriptor::new(dimensions, Some(auto_next));
        let generator     = IntegerHier::new();
        let generator     = Rc::from(RefCell::new(generator));
        let class         = "integer".to_string();
        let name          = "test hier".to_string();
        let print_opts    = None;

        let configuration =
            HierConfig { descriptor, generator, class, name, window_size, print_opts };

        Hier::new(configuration)
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_simple_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     generator    = IntegerHier::new();
        let     member_rc    = generator.make_member("test member", &None);
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

        let new_member_rc = generator.make_from_exporter(name, &None, exporter_rc);

        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);

        // Now make an actual hier instance.

        let     auto_next = 200;
        let mut hier      = make_test_hier(auto_next, None);
        let mut events    = 0;

        for i in 1..auto_next / 2 {
            hier.record_i64(i);

            events += 1;
        }

        let float = events as f64;
        let mean  = (float * (float + 1.0) / 2.0) / float;

        assert!(hier.mean() == mean);
        assert!(hier.event_count() == events);
        hier.print();
    }

    fn test_window() {
        let     auto_next   = 100;
        let     window_size = Some(1000);
        let mut hier        = make_test_hier(auto_next, window_size);
        let     period      = level_0_period();
        let     window_size = window_size.unwrap() as i64;
        let mut events      = 0 as i64;

        assert!( hier.int_extremes  ());
        assert!(!hier.float_extremes());

        for i in 0..window_size {
            let sample = i + 1;

            hier.record_i64(sample);
            events += 1;
            assert!(hier.count()   == events as u64);
            assert!(hier.min_i64() == 1            );
            assert!(hier.max_i64() == sample       );

            let level_0_pushes = (events + auto_next - 1) / auto_next;
            let level_0_all    = std::cmp::min(level_0_pushes, level_0_retain() as i64);
            let level_0_live   = std::cmp::min(level_0_pushes, level_0_period() as i64);

            assert!(hier.all_len (0) == level_0_all  as usize);
            assert!(hier.live_len(0) == level_0_live as usize);

            if hier.all_len(0) > period {
                assert!(hier.all_len(1) > 0);
            }
        }

        // Compute the expected mean of the window.

        let sum   = (window_size * (window_size + 1)) / 2;
        let sum   = sum as f64;
        let count = events as f64;
        let mean  = sum / count;

        // Check the mean and event count from the Rustics interface.

        assert!(hier.count()       == events as u64);
        assert!(hier.mean()        == mean         );
        assert!(hier.event_count() == events       );

        // Make sure that count() obeys the window_size...

        hier.record_i64(window_size + 1);
        events += 1;

        assert!(hier.count() == window_size as u64);

        {
            let current_rc = hier.current();
            let current    = current_rc.borrow();
            let histogram  = current.to_histogram();
            let histogram  = histogram.to_log_histogram().unwrap();
            let histogram  = histogram.borrow();

            let mut sum = 0;

            for sample in histogram.positive.iter() {
                sum += *sample;
            }

            let expected = events % window_size;

            assert!(expected != 0);

            println!("test_window:  got {}, expected {}", sum, expected);
            assert!(sum == expected as u64);
        }
    }

    fn test_exporter() {
        let mut exporter = IntegerExporter::new();
        let     running  = RunningInteger::new("Test Stat", &None);
        let     export   = running.export_data();

        exporter.push(export.clone());
        exporter.push(export.clone());
        exporter.push(export.clone());

        // Do a feeble test for equality.  We could set an id to do
        // a stronger test.

        assert!(exporter.count() == 3);

        let any          = exporter.as_any();
        let any_exporter = any.downcast_ref::<IntegerExporter>().unwrap();

        assert!(any_exporter.count() == 3);

        let any          = exporter.as_any_mut();
        let any_exporter = any.downcast_ref::<IntegerExporter>().unwrap();

        assert!(any_exporter.count() == 3);
    }

    #[test]
    #[should_panic]
    fn hz_panic() {
        let hier = make_test_hier(200, None);

        let _ = hier.hz();
    }

    #[test]
    fn run_tests() {
        test_simple_running_generator();
        test_window();
        test_exporter();
    }
}
