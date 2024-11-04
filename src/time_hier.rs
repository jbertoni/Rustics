//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
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
//!     // defaults for the title and printer are fine, so just pass None
//!     // for print_opts.
//!     //
//!     // The title defaults to the name and output will go to stdout.
//!
//!     let name        = "Hierarchical Time".to_string();
//!     let print_opts  = None;
//!     let window_size = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         TimeHierConfig { descriptor, name, window_size, timer, print_opts };
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
//!     // Print our data.
//!
//!     time_hier.print();
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
use super::PrintOption;
use super::running_time::RunningTime;
use super::time_window::TimeWindow;
use crate::running_integer::IntegerExporter;

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
/// for a TimeHier instance.  The window_size parameter can be
/// set to cause the Hier instance to maintain a window of the
/// last n events to be used for its Rustics reporting.

pub struct TimeHierConfig {
    pub name:        String,
    pub descriptor:  HierDescriptor,
    pub window_size: Option<usize>,
    pub timer:       TimerBox,
    pub print_opts:  PrintOption,
}

impl TimeHier {
    /// The new() function constructs a TimeHier instance, which is an
    /// implementation of HierGenerator for the RunningTime type.  Most users
    /// should just invoke new_hier() or new_hier_box().

    pub fn new(timer: TimerBox) -> TimeHier  {
        TimeHier { timer }
    }

    /// new_hier() constructs a new Hier instance from the given
    /// configuration.  It does the grunt work specific to the
    /// RunningTime type.

    pub fn new_hier(configuration: TimeHierConfig) -> Hier {
        let generator   = TimeHier::new(configuration.timer);
        let generator   = Rc::from(RefCell::new(generator));
        let class       = "time".to_string();

        let descriptor  = configuration.descriptor;
        let name        = configuration.name;
        let print_opts  = configuration.print_opts;
        let window_size = configuration.window_size;

        let config =
            HierConfig { descriptor, generator, name, window_size, class, print_opts };

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

    fn make_member(&self, name: &str, print_opts: &PrintOption) -> MemberRc {
        let member = RunningTime::new(name, self.timer.clone(), print_opts);

        Rc::from(RefCell::new(member))
    }

    fn make_window(&self, name: &str, window_size: usize, print_opts: &PrintOption)
            -> Box<dyn Rustics> {
        let window = TimeWindow::new(name, window_size, self.timer.clone(), print_opts);

        Box::new(window)
    }

    // Makes a member from a complete list of exported instances.

    fn make_from_exporter(&self, name: &str, print_opts: &PrintOption, exporter: ExporterRc) -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<IntegerExporter>().unwrap();
        let     member          = exporter_impl.make_member(name, print_opts);
        let     timer           = self.timer.clone();
        let     member          = RunningTime::from_integer(timer, print_opts, member);

        Rc::from(RefCell::new(member))
    }

    // Makes a new exporter so the Hier code can sum some RunningTime
    // instances.

    fn make_exporter(&self) -> ExporterRc {
        let exporter = IntegerExporter::new();

        Rc::from(RefCell::new(exporter))
    }

    // Pushes another instance onto the export list.  We will sum all of
    // them at some point.

    fn push(&self, exporter: &mut dyn HierExporter, member_rc: MemberRc) {
        let     exporter_any  = exporter.as_any_mut();
        let     exporter_impl = exporter_any.downcast_mut::<IntegerExporter>().unwrap();

        let     member_borrow = member_rc.borrow();
        let     member_impl   = member_borrow.as_any().downcast_ref::<RunningTime>().unwrap();

        exporter_impl.push(member_impl.export());
    }

    fn hz(&self) -> u128 {
        self.timer.borrow().hz()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;
    use crate::hier::GeneratorRc;
    use crate::tests::continuing_box;
    use crate::tests::continuing_timer_increment;

    fn level_0_period() -> usize {
        8
    }

    fn level_0_retain() -> usize {
        3 * level_0_period()
    }

    fn make_descriptor(auto_next: i64) -> HierDescriptor {
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

    fn make_time_hier(generator:  GeneratorRc, auto_next: i64, window_size: Option<usize>) -> Hier {
        let descriptor    = make_descriptor(auto_next);
        let class         = "time".to_string();
        let name          = "test hier".to_string();
        let print_opts    = None;

        let configuration =
            HierConfig { descriptor, generator, class, name, window_size, print_opts };

        Hier::new(configuration)
    }

    fn test_new_hier_box() {
        let     auto_next     = 200;
        let     descriptor    = make_descriptor(auto_next);
        let     name          = "test hier".to_string();
        let     print_opts    = None;
        let     timer         = continuing_box();
        let     window_size   = None;
        let     configuration = TimeHierConfig { descriptor, name, window_size, timer, print_opts };

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
        assert!(hier_impl.mean()        == mean  );
        assert!(hier_impl.class()       == "time");

        hier_impl.print();
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_simple_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     timer        = continuing_box();
        let     generator    = TimeHier::new(timer);
        let     member_rc    = generator.make_member("test member", &None);
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

        let new_member_rc = generator.make_from_exporter("member export", &None, exporter_rc);


        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);
        assert!(new_member.to_rustics().class() == "time");

        // Now make an actual hier instance.

        let     generator = Rc::from(RefCell::new(generator));
        let mut hier      = make_time_hier(generator, 4, None);

        let mut events = 0;

        for i in 0..100 {
            hier.record_time(i + 1);

            events += 1;
        }

        assert!(hier.event_count() == events);
        hier.print();
    }

    fn test_window() {
        let     auto_next   = 100;
        let     window_size = Some(1000);
        let     timer       = continuing_box();
        let     generator   = TimeHier::new(timer);
        let     generator   = Rc::from(RefCell::new(generator));
        let mut hier        = make_time_hier(generator, auto_next, window_size);
        let     period      = level_0_period();
        let     window_size = window_size.unwrap() as i64;
        let mut events      = 0 as i64;

        // Check time_window *_extremes.

        assert!( hier.int_extremes  ());
        assert!(!hier.float_extremes());

        for i in 0..window_size {
            let sample = i + 1;

            hier.record_time(sample);
            events += 1;
            assert!(hier.count()   == events as u64);
            assert!(hier.max_i64() == sample       );

            let level_0_pushes = (events + auto_next - 1) / auto_next;
            let level_0_all    = std::cmp::min(level_0_pushes, level_0_retain() as i64);
            let level_0_live   = std::cmp::min(level_0_pushes, level_0_period() as i64);

            assert!(hier.all_len (0) == level_0_all  as usize);
            assert!(hier.live_len(0) == level_0_live as usize);

            if hier.all_len(0) > period {
                assert!(hier.all_len(1) > 0);
            }

            assert!(hier.count()       == events as u64);
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

        // Make sure that count() matches the window_size.

        hier.record_time(window_size + 1);

        assert!(hier.count() == window_size as u64);

        // Start again and test record_event().

        let     timer           = continuing_box();
        let     generator       = TimeHier::new(timer);
        let     generator       = Rc::from(RefCell::new(generator));
        let mut hier            = make_time_hier(generator, auto_next, Some(window_size as usize));
        let     timer_increment = continuing_timer_increment();
        let mut timer_interval  = timer_increment;
        let mut total_time      = 0;
        let mut events          = 0;

        // Check the clock.

        {
            let timer     = continuing_box();
            let generator = TimeHier::new(timer);
            let timer     = continuing_box();
            let timer     = timer.borrow();

            assert!(generator.hz() == timer.hz());
        }

        for i in 1..=window_size {
            hier.record_event();
            assert!(hier.max_i64() == timer_interval );
            assert!(hier.min_i64() == timer_increment);
            assert!(hier.count()   == i as u64       );

            total_time     += timer_interval;
            timer_interval += timer_increment;
            events         += 1;
        }

        let mean = total_time as f64 / events as f64;

        assert!(hier.mean()  == mean  );
        assert!(hier.class() == "time");

        {
            let histogram = hier.to_log_histogram().unwrap();
            let histogram = histogram.borrow();

            let mut sum = 0;

            for sample in histogram.positive.iter() {
                sum += *sample;
            }

            assert!(sum == events);
        }

        hier.clear_all();
        assert!(hier.mean() == 0.0);

        hier.record_interval(&mut continuing_box());
        events = 1;

        assert!(hier.mean()  == continuing_timer_increment() as f64);
        assert!(hier.count() == events);

    }

    #[test]
    fn run_tests() {
        test_simple_running_generator();
        test_new_hier_box();
        test_window();
    }
}
