//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

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
use super::running_integer::RunningInteger;
use crate::running_integer::RunningExporter;

use crate::Hier;
use crate::HierBox;
use crate::HierDescriptor;
use crate::HierConfig;
use crate::HierGenerator;
use crate::HierMember;
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

// IntegerHier provides an interface from the Hier code to
// the RunningInteger code.

#[derive(Default)]
pub struct IntegerHier {
}

pub struct IntegerHierConfig {
    pub descriptor:  HierDescriptor,
    pub name:        String,
    pub title:       String,
    pub printer:     PrinterBox,
}

impl IntegerHier {
    pub fn new_raw() -> IntegerHier  {
        IntegerHier { }
    }

    // Create a new Hier object from the given configuration.
    // This routine does the grunt work specific to the
    // RunningInteger type.

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

    pub fn new_hier_box(configuration: IntegerHierConfig) -> HierBox {
        let hier = IntegerHier::new_hier(configuration);

        Arc::from(Mutex::new(hier))
    }
}

// These are the functions that the Hier struct needs implemented
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

    fn push(&self, exporter: ExporterRc, member_rc: MemberRc) {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_any    = exporter_borrow.as_any_mut();
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
        let title         = "test hier".to_string();
        let printer       = stdout_printer();

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

        generator.push(exporter_clone, member_rc);

        let name    = "member export";
        let printer = stdout_printer();

        let new_member_rc = generator.make_from_exporter(name, printer, exporter_rc);

        // See that the new member matches expectations.

        let new_member = new_member_rc.borrow();

        assert!(new_member.to_rustics().count() == 1);
        assert!(new_member.to_rustics().mean()  == value as f64);

        // Now make an actual hier struct.

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
