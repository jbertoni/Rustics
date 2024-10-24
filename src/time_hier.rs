//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!
//! ## Type
//!
//! * TimeHier
//!   * This type implements hierarchical statistics using the RunningTime
//!     type.  See the running_time module for details on that type.
//!
//!   * The functions TimeHier::new_hier and TimeHier::new_hier_box are
//!     wrappers for the Hier constructor and do the initialization
//!     specific to the TimeHier type.  They are the preferred interface
//!     for creating a Hier instance that use RunningTime instances.
//!
//!   * See the integer_hier module for more details on hierarchical
//!     statistics.
//!
//!```
//!     // This example is based on the code in IntegerHier.
//!
//!     use rustics::Rustics;
//!     use rustics::stdout_printer;
//!     use rustics::hier::Hier;
//!     use rustics::hier::HierDescriptor;
//!     use rustics::hier::HierDimension;
//!     use rustics::hier::HierIndex;
//!     use rustics::hier::HierSet;
//!     use rustics::time_hier::TimeHier;
//!     use rustics::time_hier::TimeHierConfig;
//!     use rustics::time::Timer;
//!     use rustics::time::DurationTimer;
//!
//!     // Make a descriptor of the first level.  We have chosen to sum
//!     // 1000 level 0 RunningTime instances into one level 1 RunningTime
//!     // instance.  This level is large, so we will keep only 1000
//!     // level 0 instances in the window.
//!
//!     let dimension_0 = HierDimension::new(1000, 1000);
//!
//!     // At level 1, we want to sum 100 level 1 instances into one level
//!     // 2 instance.  This level is smaller, so let's retain 200
//!     // RunningTime instances here.
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
//!     // record 2000 time samples into each level 0 RunningTime instance.
//!
//!     let auto_advance = Some(2000);
//!     let descriptor   = HierDescriptor::new(dimensions, auto_advance);
//!
//!     // Use DurationTimer for the clock.
//!
//!     let timer = DurationTimer::new_box();
//!
//!     // Now specify some parameters used by Hier to do printing.  The
//!     // defaults for the title and printer are fine, so just pass None.
//!     // The title defaults to the name and output will go to stdout.
//!
//!     let name    = "Hierarchical Time".to_string();
//!     let title   = None;
//!     let printer = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         TimeHierConfig { descriptor, name, timer, title, printer };
//!
//!     // Now make the Hier instance and lock it.
//!
//!     let     time_hier = TimeHier::new_hier_box(configuration);
//!     let mut time_hier = time_hier.lock().unwrap();
//!
//!     // Now record some events with boring data.
//!
//!     let mut events   = 0;
//!     let auto_advance = auto_advance.unwrap();
//!
//!     for i in  0..auto_advance {
//!         events += 1;
//!         time_hier.record_event();
//!     }
//!
//!     // We have just completed the first level 0 instance, but the
//!     // implementation creates the next instance only when it has data
//!     // to record, so there should be only one level zero instance,
//!     // and nothing at level 1 or level 2.
//!
//!     assert!(time_hier.event_count() == events);
//!     assert!(time_hier.count()       == events as u64);
//!     assert!(time_hier.live_len(0)   == 1     );
//!     assert!(time_hier.live_len(1)   == 0     );
//!     assert!(time_hier.live_len(2)   == 0     );
//!
//!     // Now record a sample to force the creation of the second level
//!     // 1 instance.
//!
//!     events += 1;
//!     time_hier.record_time(10);
//!
//!     // The new level 0 instance should have only one event recorded.
//!     // The Rustics implementatio for Hier returns the data in the
//!     // current level 0 instance, so check it.
//!
//!     assert!(time_hier.event_count() == events);
//!     assert!(time_hier.count()       == 1     );
//!     assert!(time_hier.live_len(0)   == 2     );
//!     assert!(time_hier.live_len(1)   == 0     );
//!     assert!(time_hier.live_len(2)   == 0     );
//!
//!     let events_per_level_1 =
//!         auto_advance * dimension_0.period() as i64;
//!
//!     // Use the finish() method this time.  It uses the clock
//!     // directly.  This approach works if multiple threads
//!     // are using the Hier instance.
//!
//!     let timer = DurationTimer::new_box();
//!
//!     for _i in events..events_per_level_1 {
//!         time_hier.record_time(timer.borrow_mut().finish());
//!         events += 1;
//!     }
//!
//!     // Check the state again.  We need to record one more event to
//!     // cause the summation at level 0 into level 1.
//!
//!     let expected_live  = dimension_0.period();
//!     let expected_count = auto_advance as u64;
//!
//!     assert!(time_hier.event_count() == events        );
//!     assert!(time_hier.count()       == expected_count);
//!     assert!(time_hier.live_len(0)   == expected_live );
//!     assert!(time_hier.live_len(1)   == 0             );
//!     assert!(time_hier.live_len(2)   == 0             );
//!
//!     time_hier.record_time(42);
//!     events += 1;
//!
//!     assert!(time_hier.live_len(1)   == 1     );
//!     assert!(time_hier.event_count() == events);
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
//!     time_hier.print_index_opts(index, None, None);
//!```

//
// This module provides the interface between RunningTime and the Hier
// code.
//

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use super::Rustics;
use super::Histogram;
use super::HierBox;
use super::TimerBox;
use super::PrinterBox;
use super::PrinterOption;
use super::running_time::RunningTime;
use crate::running_integer::RunningExporter;

use crate::Hier;
use crate::HierDescriptor;
use crate::HierConfig;
use crate::HierGenerator;
use crate::HierMember;
use crate::HierExporter;
use crate::ExporterRc;
use crate::MemberRc;

// Provide for downcasting from a Hier member to a Rustics
// type or "dn Any" to get to the RunningTime code.

impl HierMember for RunningTime {
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

/// TimeHier provides an interface from the Hier code to
/// the RunningTime code.
///
/// See the module comments for sample code.

#[derive(Clone)]
pub struct TimeHier {
    timer:  TimerBox,
}

/// TimeHierConfig is used to pass the configuration parameters
/// for a TimeHier instance.

#[derive(Clone)]
pub struct TimeHierConfig {
    pub name:        String,
    pub descriptor:  HierDescriptor,
    pub timer:       TimerBox,
    pub title:       Option<String>,
    pub printer:     PrinterOption,
}

impl TimeHier {
    /// The new_raw() function constructs a TimeHier instance, which is an
    /// implementation of HierGenerator for the RunningTime type.  Most users
    /// should just invoke new_hier() or new_hier_box().

    pub fn new_raw(timer: TimerBox) -> TimeHier  {
        TimeHier { timer }
    }

    /// new_hier() constructs a new Hier instance from the given
    /// configuration.  It does the grunt work specific to the
    /// RunningTime type.

    pub fn new_hier(configuration: TimeHierConfig) -> Hier {
        let generator  = TimeHier::new_raw(configuration.timer);
        let generator  = Rc::from(RefCell::new(generator));
        let class      = "integer".to_string();

        let descriptor = configuration.descriptor;
        let name       = configuration.name;
        let title      = configuration.title;
        let printer    = configuration.printer;

        let config =
            HierConfig { descriptor, generator, name, title, class, printer };

        Hier::new(config)
    }

    /// new_hier_box() returns a Hier instance as an Arc<Mutex<Hier>>
    /// for multithreaded access.

    pub fn new_hier_box(configuration: TimeHierConfig) -> HierBox {
        let hier = TimeHier::new_hier(configuration);

        Arc::from(Mutex::new(hier))
    }
}

// This impl provides the thus bridge between "impl RunningTime"
// and the Hier code.  This cdoe is of interest mainly to developers
// who are creating a custom type and need examples.

impl HierGenerator for TimeHier {
    // Creates a member with the given name and printer.

    fn make_member(&self, name: &str, printer: PrinterBox) -> MemberRc {
        let member = RunningTime::new(name, self.timer.clone(), Some(printer));

        Rc::from(RefCell::new(member))
    }

    // Makes a member from a complete list of exported instances.

    fn make_from_exporter(&self, name: &str, printer: PrinterBox, exporter: ExporterRc) -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<RunningExporter>().unwrap();
        let     member          = exporter_impl.make_member(name, printer.clone());
        let     timer           = self.timer.clone();
        let     member          = RunningTime::from_integer(timer, printer, member);

        Rc::from(RefCell::new(member))
    }

    // Makes a new exporter so the Hier code can sum some RunningTime
    // instances.

    fn make_exporter(&self) -> ExporterRc {
        let exporter = RunningExporter::new();

        Rc::from(RefCell::new(exporter))
    }

    // Pushes another instance onto the export list.  We will sum all of
    // them at some point.

    fn push(&self, exporter: &mut dyn HierExporter, member_rc: MemberRc) {
        let     exporter_any  = exporter.as_any_mut();
        let     exporter_impl = exporter_any.downcast_mut::<RunningExporter>().unwrap();

        let     member_borrow = member_rc.borrow();
        let     member_impl   = member_borrow.as_any().downcast_ref::<RunningTime>().unwrap();

        exporter_impl.push(member_impl.export());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdout_printer;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;
    use crate::hier::GeneratorRc;
    use crate::tests::continuing_box;
    use crate::tests::continuing_timer_increment;

    fn make_descriptor(auto_next: i64) -> HierDescriptor {
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

        HierDescriptor::new(dimensions, Some(auto_next))
    }

    fn make_hier_gen(generator:  GeneratorRc) -> Hier {
        let descriptor    = make_descriptor(4);
        let class         = "integer".to_string();
        let name          = "test hier".to_string();
        let title         = None;
        let printer       = None;

        let configuration = HierConfig { descriptor, generator, class, name, title, printer };

        Hier::new(configuration)
    }

    fn test_new_hier_box() {
        let     auto_next     = 200;
        let     descriptor    = make_descriptor(auto_next);
        let     name          = "test hier".to_string();
        let     title         = None;
        let     printer       = Some(stdout_printer());
        let     timer         = continuing_box();
        let     configuration = TimeHierConfig { descriptor, name, timer, title, printer };

        let     hier          = TimeHier::new_hier_box(configuration);
        let mut hier_impl     = hier.lock().unwrap();

        // Now just record a few events.

        let mut events = 0;
        let mut sum    = 0;

        for i in 0..auto_next / 2 {
            hier_impl.record_event();

            // Make sure that this event was recorded properly.

            let expected = (i + 1) * continuing_timer_increment();

            assert!(hier_impl.max_i64() == expected);
            sum += expected;

            // Now try record_time()

            hier_impl.record_time(i);

            sum += i;
            events += 2;
        }

        let mean  = sum as f64 / events as f64;

        // Check that the event count and mean match our
        // expectations.

        assert!(hier_impl.event_count() == events);
        assert!(hier_impl.mean() == mean);
        hier_impl.print();
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_simple_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     timer        = continuing_box();
        let     generator    = TimeHier::new_raw(timer);
        let     printer      = stdout_printer();
        let     member_rc    = generator.make_member("test member", printer);
        let     member_clone = member_rc.clone();
        let mut member       = member_clone.borrow_mut();
        let     value        = 42;

        member.to_rustics_mut().record_time(value);

        assert!(member.to_rustics().count() == 1);
        assert!(member.to_rustics().mean()  == value as f64);

        // Drop the lock on the member.

        drop(member);

        // Now try try making an exporter and check basic sanity of as_any_mut.

        let exporter_rc     = generator.make_exporter();

        // Push the member's numbers onto the exporter.

        generator.push(&mut *exporter_rc.borrow_mut(), member_rc);

        let new_member_rc = generator.make_from_exporter("member export", stdout_printer(), exporter_rc);


        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);

        // Now make an actual hier instance.

        let     generator = Rc::from(RefCell::new(generator));
        let mut hier      = make_hier_gen(generator);

        let mut events = 0;

        for i in 0..100 {
            hier.record_time(i + 1);

            events += 1;
        }

        assert!(hier.event_count() == events);
        hier.print();
    }

    #[test]
    fn run_tests() {
        test_simple_running_generator();
        test_new_hier_box();
    }
}
