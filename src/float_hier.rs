//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//! ## Type
//!
//! * FloatHier
//!     * This type implements hierarchical statistics using the
//!       RunningFloat type, q.v.  It is substantially identical
//!       to IntegerHier.
//!
//!     * Each level uses a Window instance containing a configurable
//!       number of RunningFloat instances.  See the window module
//!       documentation for more information on how the windows work.
//!
//!     * Level 0 RunningFloat instances are used to collect data.
//!       Each instance collects a configurable number of samples.
//!       After that number of samples are gathered, a new
//!       RunningFloat instance is pushed into the window.
//!
//!     * When a configurable number of level 0 instances have been
//!       collected into the window, they are summed into one level 1
//!       RunningFloat instance.
//!
//!     * In general, a Rustics intance at level j is a sum of of i
//!       instances from level j - 1, where i is configured per level.
//!
//!     * Each window retains RunningFloat instances that have
//!       already been summed, in case they are wanted for queries.
//!       The total window size is configured per level, and limits
//!       the number of retained RunningFloat instances.
//!
//! ## Example
//!```
//!    // This example is largely identical to the IntegerHier example.
//!
//!     use rustics::Rustics;
//!     use rustics::stdout_printer;
//!     use rustics::hier::Hier;
//!     use rustics::hier::HierDescriptor;
//!     use rustics::hier::HierDimension;
//!     use rustics::hier::HierIndex;
//!     use rustics::hier::HierSet;
//!     use rustics::float_hier::FloatHier;
//!     use rustics::float_hier::FloatHierConfig;
//!
//!     // Make a descriptor of the first level.  We have chosen to sum
//!     // 1000 level 0 RunningFloat instances into one level 1
//!     // RunningFloat instance.  This level is large, so we will keep
//!     // only 1000 level 0 instances in the window.
//!
//!     let dimension_0 = HierDimension::new(1000, 1000);
//!
//!     // At level 1, we want to sum 100 level 1 instances into one level
//!     // 2 instance.  This level is smaller, so let's retain 200
//!     // RunningFloat instances here.
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
//!     // record 2000 events into each level 0 RunningFloat instance.
//!
//!     let auto_advance = Some(2000);
//!     let descriptor   = HierDescriptor::new(dimensions, auto_advance);
//!
//!     // Now specify some parameters used by Hier to do printing.  The
//!     // defaults for the title and printer are fine, so just pass None.
//!     // The title defaults to the name and output will go to stdout.
//!     // Don't configure a window for this example.
//!
//!     let name        = "hierarchical float".to_string();
//!     let print_opts  = None;
//!     let window_size = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         FloatHierConfig {
//!             descriptor, name, window_size, print_opts
//!         };
//!
//!     // Now make the Hier instance and lock it.
//!
//!     let     float_hier = FloatHier::new_hier_box(configuration);
//!     let mut float_hier = float_hier.lock().unwrap();
//!
//!     // Now record some events with boring data.
//!
//!     let mut events   = 0;
//!     let auto_advance = auto_advance.unwrap();
//!
//!     for i in  0..auto_advance {
//!         events += 1;
//!         float_hier.record_f64(i as f64 + 10.0);
//!     }
//!
//!     // We have just completed the first level 0 instance, but the
//!     // implementation creates the next instance only when it has data
//!     // to record, so there should be only one level zero instance,
//!     // and nothing at level 1 or level 2.
//!
//!     assert!(float_hier.event_count() == events);
//!     assert!(float_hier.count()       == events as u64);
//!     assert!(float_hier.live_len(0)   == 1     );
//!     assert!(float_hier.live_len(1)   == 0     );
//!     assert!(float_hier.live_len(2)   == 0     );
//!
//!     // Now record some data to force the creation of the second level
//!     // 1 instance.
//!
//!     events += 1;
//!     float_hier.record_f64(10.0);
//!
//!     // The new level 0 instance should have only one event recorded.
//!     // The Rustics implementatio for Hier returns the data in the
//!     // current level 0 instance, so check it.
//!
//!     assert!(float_hier.event_count() == events);
//!     assert!(float_hier.count()       == 1     );
//!     assert!(float_hier.live_len(0)   == 2     );
//!     assert!(float_hier.live_len(1)   == 0     );
//!     assert!(float_hier.live_len(2)   == 0     );
//!
//!     let events_per_level_1 =
//!         auto_advance * dimension_0.period() as i64;
//!
//!     for i in events..events_per_level_1 {
//!         float_hier.record_f64(i as f64);
//!         events += 1;
//!     }
//!
//!     // Check the state again.  We need to record one more event to
//!     // cause the summation at level 0 into level 1.
//!
//!     let expected_live  = dimension_0.period();
//!     let expected_count = auto_advance as u64;
//!
//!     assert!(float_hier.event_count() == events        );
//!     assert!(float_hier.count()       == expected_count);
//!     assert!(float_hier.live_len(0)   == expected_live );
//!     assert!(float_hier.live_len(1)   == 0             );
//!     assert!(float_hier.live_len(2)   == 0             );
//!
//!     float_hier.record_f64(42.0);
//!     events += 1;
//!
//!     assert!(float_hier.live_len(1)   == 1     );
//!     assert!(float_hier.event_count() == events);
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
//!     float_hier.print_index_opts(index, None, None);
//!```

//
// This module provides the interface between RunningFloat and the Hier
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
use super::running_float::RunningFloat;
use crate::running_float::FloatExporter;
use super::float_window::FloatWindow;

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
// type or "dn Any" to get to the RunningFloat code.

impl HierMember for RunningFloat {
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

/// FloatHier provides an interface from the Hier code to the
/// RunningFloat impl code that is not in methods.  Most users
/// should construct a Hier instance via functions like new_hier()
/// and new_hier_box() that do the type-specific initialization.
///
/// See the module comments for a sample program.

#[derive(Default)]
pub struct FloatHier {
}

/// FloatHierConfig is used to pass the constructor parameters
/// for a Hier instance that uses the RunningFloat type for
/// recording and combining data.  The window_size parameter can
/// be set to cause the Hier instance to maintain a window of the
/// last n events to be used for its Rustics reporting.

pub struct FloatHierConfig {
    pub descriptor:  HierDescriptor,
    pub name:        String,
    pub print_opts:  PrintOption,
    pub window_size: Option<usize>,
}

impl FloatHier {
    pub fn new() -> FloatHier  {
        FloatHier { }
    }

    /// new_hier() creates a new Hier instance from the given
    /// configuration.  This function does the grunt work specific
    /// to the RunningFloat type.

    pub fn new_hier(configuration: FloatHierConfig) -> Hier {
        let generator    = FloatHier::new();
        let generator    = Rc::from(RefCell::new(generator));
        let class        = "float".to_string();

        let descriptor   = configuration.descriptor;
        let name         = configuration.name;
        let print_opts   = configuration.print_opts;
        let window_size  = configuration.window_size;

        let config =
            HierConfig { 
                descriptor, generator, name, window_size, class, print_opts
            };

        Hier::new(config)
    }

    /// new_hier_box() uses new_hier() to create a Hier instance and
    /// returns it as an Arc<Mutex<Hier>> for multi-threaded
    /// use.

    pub fn new_hier_box(configuration: FloatHierConfig) -> HierBox {
        let hier = FloatHier::new_hier(configuration);

        Arc::from(Mutex::new(hier))
    }
}

// These are the methods that the Hier instance needs implemented
// for a given statistics type that are not specific to an instance
// of that type.  It's thus the bridge between "impl RunningFloat"
// and the Hier code.

impl HierGenerator for FloatHier {
    fn make_member(&self, name: &str, print_opts: &PrintOption) -> MemberRc {
        let member = RunningFloat::new(name, print_opts);

        Rc::from(RefCell::new(member))
    }

    fn make_window(&self, name: &str, window_size: usize, print_opts: &PrintOption)
            -> Box<dyn Rustics> {
        let window = FloatWindow::new(name, window_size, print_opts);

        Box::new(window)
    }

    // Make a member from a complete list of exported statistics.

    fn make_from_exporter(&self, name: &str, print_opts: &PrintOption, exporter: ExporterRc)
            -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
        let     exporter_impl   = exporter_any.downcast_mut::<FloatExporter>().unwrap();
        let     member          = exporter_impl.make_member(name, print_opts);

        Rc::from(RefCell::new(member))
    }

    fn make_exporter(&self) -> ExporterRc {
        let exporter = FloatExporter::new();

        Rc::from(RefCell::new(exporter))
    }

    // Push another instance onto the export list.  We will sum all of
    // them at some point.

    fn push(&self, exporter: &mut dyn HierExporter, member_rc: MemberRc) {
        let exporter_any   = exporter.as_any_mut();
        let exporter_impl  = exporter_any.downcast_mut::<FloatExporter>().unwrap();

        let member_borrow  = member_rc.borrow();
        let member_any     = member_borrow.as_any();
        let member_impl    = member_any.downcast_ref::<RunningFloat>().unwrap();

        exporter_impl.push(member_impl.export_data());
    }

    fn hz(&self) -> u128 {
        panic!("FloatHier::hz:  not supported");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hier::HierDescriptor;
    use crate::hier::HierDimension;
    use crate::stdout_printer;

    fn level_0_period() -> usize {
        8
    }

    fn level_0_retain() -> usize {
        3 * level_0_period()
    }

    fn make_test_hier(auto_next: i64, window_size: Option<usize>) -> HierBox {
        let     levels         = 4;
        let     level_0_period = level_0_period();
        let     dimension      = HierDimension::new(level_0_period, level_0_retain());
        let mut dimensions     = Vec::<HierDimension>::with_capacity(levels);

        assert!(dimension.period()    == level_0_period  );
        assert!(dimension.retention() == level_0_retain());

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
        let name          = "test hier".to_string();
        let print_opts    = None;

        let configuration =
            FloatHierConfig { descriptor, name, window_size, print_opts };

        FloatHier::new_hier_box(configuration)
    }

    // Do a minimal liveness test of the generic hier implementation.

    fn test_simple_running_generator() {
        //  First, just make a generator and a member, then record one event.

        let     generator    = FloatHier::new();
        let     member_rc    = generator.make_member("test member", &None);
        let     member_clone = member_rc.clone();
        let mut member       = member_clone.borrow_mut();
        let     value        = 42.0;

        member.to_rustics_mut().record_f64(value);

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
        let     hier      = make_test_hier(auto_next, None);
        let mut hier      = hier.lock().unwrap();
        let mut events    = 0;

        for i in 1..auto_next / 2 {
            hier.record_f64(i as f64);

            events += 1;
        }

        let float = events as f64;
        let mean  = (float * (float + 1.0) / 2.0) / float;

        assert!(hier.mean()        == mean);
        assert!(hier.event_count() == events);
        assert!(hier.min_f64()     == 1.0  );
        assert!(hier.max_f64()     == float);

        hier.print();

        hier.record_f64(f64::NAN);
        assert!(hier.to_float_histogram().unwrap().borrow().samples == events as usize);

        hier.clear_histogram();
        hier.print();

        assert!(hier.to_float_histogram().unwrap().borrow().samples == 0);
    }

    fn test_window() {
        let     printer   = stdout_printer();
        let     printer   = &mut *printer.lock().unwrap();
        let     auto_next   = 100;
        let     window_size = Some(1000);
        let     hier        = make_test_hier(auto_next, window_size);
        let mut hier        = hier.lock().unwrap();
        let     period      = level_0_period();
        let     window_size = window_size.unwrap() as i64;
        let mut events      = 0 as i64;

        assert!(!hier.int_extremes  ());
        assert!( hier.float_extremes());

        for i in 0..window_size {
            let sample = i + 1;

            hier.record_f64(sample as f64);
            events += 1;
            assert!(hier.count()   == events as u64);
            assert!(hier.min_f64() == 1.0          );
            assert!(hier.max_f64() == sample as f64);

            let level_0_pushes = (events + auto_next - 1) / auto_next;
            let level_0_all    = std::cmp::min(level_0_pushes, level_0_retain() as i64);
            let level_0_live   = std::cmp::min(level_0_pushes, level_0_period() as i64);

            assert!(hier.all_len (0) == level_0_all  as usize);
            assert!(hier.live_len(0) == level_0_live as usize);

            if hier.all_len(0) > period {
                assert!(hier.all_len(1) > 0);
            }
        }

        {
            let histogram = hier.to_float_histogram().unwrap();
            let histogram = histogram.borrow();
            
            let mut sum = 0;

            for sample in histogram.positive.iter() {
                sum += *sample;
            }

            assert!(sum == events as u64);
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

        hier.print_histogram(printer);

        // Make sure that count() obeys the window_size...

        hier.record_f64(window_size as f64 + 1.0);
        
        assert!(hier.count() == window_size as u64);

        // See whether we can get back to a member.

        let     member_rc = hier.current();
        let     member    = &mut *member_rc.borrow_mut();
        let     histogram = member.to_histogram();

        member.to_rustics().print();
        histogram.print_histogram(printer);
        member.to_rustics_mut().record_f64(1.0);

        let _any_mut = member.as_any_mut();
    }

    #[test]
    #[should_panic]
    fn test_hz() {
        let     auto_next   = 200;
        let     window_size = None;
        let     hier        = make_test_hier(auto_next, window_size);
        let     hier        = hier.lock().unwrap();

        let _hz = hier.hz();
    }

    #[test]
    #[should_panic]
    fn test_log_histogram() {
        let     auto_next   = 200;
        let     window_size = None;
        let     hier        = make_test_hier(auto_next, window_size);
        let     hier        = hier.lock().unwrap();

        let _ = hier.to_log_histogram().unwrap();
    }

    #[test]
    fn run_tests() {
        test_simple_running_generator();
        test_window();
    }
}
