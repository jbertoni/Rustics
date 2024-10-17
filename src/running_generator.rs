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

use super::Rustics;
use super::PrinterBox;
use super::RunningInteger;
use super::RunningExporter;

use crate::Hier;
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
}

// RunningGenerator provides an interface from the Hier code to
// the RunningInteger code.

#[derive(Default)]
pub struct RunningGenerator {
}

pub struct RunningHierConfig {
    pub descriptor:  HierDescriptor,
    pub name:        String,
    pub title:       String,
    pub printer:     PrinterBox,
}

impl RunningGenerator {
    pub fn new() -> RunningGenerator  {
        RunningGenerator { }
    }

    // Create a new Hier object from the given configuration.
    // This routine does the grunt work specific to the
    // RunningInteger type.

    pub fn new_hier(configuration: RunningHierConfig) -> Hier {
        let generator  = RunningGenerator::new();
        let generator  = Rc::from(RefCell::new(generator));
        let class      = "integer".to_string();

        let descriptor = configuration.descriptor;
        let name       = configuration.name;
        let title      = configuration.title;
        let printer    = configuration.printer;

        let config = HierConfig { descriptor, generator, name, title, class, printer };

        Hier::new(config)
    }
}

// These are the functions that the Hier struct needs implemented
// for a given statistic type that are not specific to a member
// of that type.  It's thus the bridge between "impl RunningInteger"
// and the Hier code.

impl HierGenerator for RunningGenerator {
    fn make_member(&self, name: &str, printer: PrinterBox) -> MemberRc {
        let member = RunningInteger::new(name, Some(printer));

        Rc::from(RefCell::new(member))
    }

    // Make a member from a complete list of exported statistics.

    fn make_from_exporter(&self, name: &str, printer: PrinterBox, exporter: ExporterRc) -> MemberRc {
        let mut exporter_borrow = exporter.borrow_mut();
        let     exporter_impl   = exporter_borrow.as_any_mut().downcast_mut::<RunningExporter>().unwrap();
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
        let     exporter_impl   = exporter_borrow.as_any_mut().downcast_mut::<RunningExporter>().unwrap();

        let     member_borrow = member_rc.borrow();
        let     member_impl   = member_borrow.as_any().downcast_ref::<RunningInteger>().unwrap();

        exporter_impl.push(member_impl.export());
    }
}
