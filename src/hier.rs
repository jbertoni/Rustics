//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkeley 3-clause,
//  and MIT licenses.
//

//!
//! ## Type
//!
//! * Hier
//!     * Hier implements a hierarchy of Rustics instances for collecting and analyzing a single
//!       sample stream.
//!
//!     * See the library comments (lib.rs) for an overview of how this type works.
//!
//!     * Hier is a framework class that should be instantiated for a concrete Rustics type
//!       via functions like IntegerHier::new_hier, FloatHier::new_hier, or TimeHier::new_hier.
//!       The example uses IntegerHier, which uses RunningInteger as the underlying Rustics type.
//!
//!     * Hier implements the Rustics interface, and through that provides statistics from the
//!       either the current level 0 Rustics instance, i.e., statistics on the newest samples,
//!       or from an optionally configured window of the last n events, as specified by the
//!       window_size parameter in HierConfig.  This window is implemented using a type such as
//!       TimeWindow, FloatWindow, or IntegerWindow.
//!
//! ## Example
//!```
//!     use std::sync::Arc;
//!     use std::sync::Mutex;
//!     use rustics::Rustics;
//!     use rustics::arc_box;
//!     use rustics::hier_item;
//!     use rustics::hier::Hier;
//!     use rustics::hier::HierDescriptor;
//!     use rustics::hier::HierDimension;
//!     use rustics::hier::HierIndex;
//!     use rustics::hier::HierSet;
//!     use rustics::integer_hier::IntegerHier;
//!     use rustics::integer_hier::IntegerHierConfig;
//!     use rustics::arc_sets::ArcSet;
//!
//!     // Make a descriptor of level zero.  We choose to sum 1000
//!     // level 0 RunningInteger instances into one level 1
//!     // RunningInteger instance.  We will keep only 1000 level 0
//!     // instances in the window as that seems large enough for a
//!     // window back in time.
//!
//!     let dimension_0 = HierDimension::new(1000, 1000);
//!
//!     // At level 1, we want to sum 100 level 1 instances into one
//!     // level 2 instance.  Let's retain 200 RunningInteger instances
//!     // at level 1.
//!
//!     let dimension_1 = HierDimension::new(100, 200);
//!
//!     // Level two isn't summed, so the period isn't used.
//!     // Let's pretend this level isn't used much, so retain
//!     // only 100 instances in it.
//!
//!     let dimension_2 = HierDimension::new(0, 100);
//!
//!     // Now create the Vec.
//!
//!     let dimensions =
//!         vec![ dimension_0, dimension_1, dimension_2 ];
//!
//!     // Now create the entire descriptor for the hier constructor.
//!     // Let's record 2000 events into each level 0 RunningInteger
//!     // instance.
//!
//!     let auto_advance = Some(2000);
//!     let descriptor   = HierDescriptor::new(dimensions, auto_advance);
//!
//!     // Now create some items used by Hier to do printing.  The
//!     // defaults for printing are fine for an example, so just pass
//!     // None.  By default, the title is the name and output is to stdout.
//!     // Don't configure a window.
//!
//!     let name        = "test hierarchical integer".to_string();
//!     let print_opts  = None;     // default to stdout
//!     let window_size = None;
//!
//!     // Finally, create the configuration description for the
//!     // constructor.
//!
//!     let configuration =
//!         IntegerHierConfig { descriptor, name, window_size, print_opts };
//!
//!     // Now make the Hier instance.
//!
//!     let mut integer_hier = IntegerHier::new_hier(configuration);
//!
//!     // Now record some events with test data samples.
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
//!     // The first level 0 instance is ready to be retired, but the
//!     // implementation creates the next instance only when it has data
//!     // to record, so there should be only one level zero instance,
//!     // and nothing at level 1 or level 2.
//!     //
//!     // event_count() returns all events seen by the integer_hier
//!     // instance from creation onward.  At this point, it should
//!     // still match our first level 0 Rustics count.
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
//!     // The Rustics implementation for Hier returns the data in the
//!     // current level 0 instance, so check it.
//!
//!     assert!(integer_hier.count() == 1);
//!
//!     // Record enough events to fill a level 1 summary.  It will not
//!     // be created yet, though.  That occurs when we start the next
//!     // level 0 batch, i.e., retire the current level 0 instance.
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
//!     // Sum the current live entries in level 0.
//!
//!     let     level   = 0;
//!     let     count   = integer_hier.live_len(0);
//!     let mut addends = Vec::new();
//!
//!     // First create a vector of indices.
//!
//!     for i in 0..count {
//!         addends.push(HierIndex::new(HierSet::Live, level, i));
//!     }
//!
//!     // Now compute the sum and print it.
//!
//!     let sum = integer_hier.sum(addends, "Level 0 Summary");
//!
//!     let sum =
//!         match sum {
//!             (Some(member), count)  =>
//!                 { member }
//!             (None,         count)  =>
//!                 { panic!("The sum wasn't created"); }
//!         };
//!
//!     let borrow  = hier_item!(sum);
//!     let rustics = borrow.to_rustics();
//!
//!     rustics.print();
//!
//!     // Use a Hier instance in a set.  Don't bother with size hints
//!     // or custom printing.
//!
//!     let     set_box = ArcSet::new_box("Test Set", 0, 0, &None);
//!     let mut set     = set_box.lock().unwrap();
//!
//!     // Add the Hier instance and call print().  We need to drop the
//!     // drop the lock on the Hier instance.
//!
//!     let integer_hier = arc_box!(integer_hier);
//!
//!     set.add_member(integer_hier.clone());
//!     set.print();
//!```

use std::rc::Rc;
use std::any::Any;

use super::Rustics;
use super::Histogram;
use super::ExportStats;
use super::Printer;
use super::PrinterBox;
use super::PrinterOption;
use super::PrintOption;
use super::LogHistogramBox;
use super::FloatHistogramBox;
use super::parse_print_opts;
use super::TimerBox;
use super::window::Window;
use super::printer_mut;
use super::timer_mut;
use std::cell::RefCell;

pub type MemberRc    = Rc<RefCell<dyn HierMember   >>;
pub type GeneratorRc = Rc<RefCell<dyn HierGenerator>>;
pub type ExporterRc  = Rc<RefCell<dyn HierExporter >>;

/// Converts a Rustics instance in the shareable form
/// for use by the Hier code.

#[macro_export]
macro_rules! hier_box { ($x:expr) => { Rc::from(RefCell::new($x)) } }

/// Converts a Hier member into a mutable Rustics or
/// subset reference.

#[macro_export]
macro_rules! hier_item_mut { ($x:expr) => { &mut *$x.borrow_mut() } }

/// Converts a Hier member into a Rustics or subset
/// reference.

#[macro_export]
macro_rules! hier_item { ($x:expr) => { &*$x.borrow() } }

/// HierDescriptor is used to describe the configuration of
/// a hierarchy to a constructor like new().

pub struct HierDescriptor {
    dimensions:     Vec<HierDimension>,
    auto_next:      i64,
}

impl HierDescriptor {
    pub fn new(dimensions: Vec<HierDimension>, auto_next: Option<i64>) -> HierDescriptor {
        let auto_next = auto_next.unwrap_or(0);

        if auto_next < 0 {
            panic!("HierDescriptor::new:  The auto_next value can't be negative.");
        }

        HierDescriptor { dimensions, auto_next }
    }
}

// This type is used to describe one level of the Rustics hierarchy.
// "period" specifies the number of instances in this window that
// are combined (summed) to create one higher-level rustics instance.
//
// "retention" specifies the total number of instances to keep
// around for queries.  It must be at least "period" instances, but
// can be more to keep more history.

/// HierDimension is used to specify the configuration of one level
/// in a Hier instance.

#[derive(Clone, Copy)]
pub struct HierDimension {
    period:     usize,   // the number of instances to be summed for the next level
    retention:  usize,   // the number of instances to retain for queries.
}

impl HierDimension {
    pub fn new(period: usize, retention: usize) -> HierDimension {
        if retention < period {
            panic!("HierDimension::new:  The retention count is too small.");
        }

        HierDimension { period, retention }
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn retention(&self) -> usize {
        self.retention
    }
}

/// HierIndex allows users to specify an index into a Hier
/// instance in order to look at any Rustics instance therein.

#[derive(Clone, Copy)]
pub struct HierIndex {
    set:   HierSet,
    level: usize,
    which: usize,
}

/// HierSet is used to refer to the specific subset of a level in the
/// hierarchy to be indexed when using a HierIndex instance.  The user can
/// choose to look at all of the instances, or only the newest entries,
/// the live set.

#[derive(Clone, Copy)]
pub enum HierSet {
    All,
    Live,
}

/// HierIndex is used to refer to a specific Rustics instance in a
/// Hier instance.

impl HierIndex {
    pub fn new(set: HierSet, level: usize, which: usize) -> HierIndex {
        HierIndex { set, level, which }
    }
}

// The exporter needs to be downcast to be used, so
// provide that interface.

/// The HierExporter trait defines the interface for creating a Rustics
/// instance that is a sum of other instances.  It can be used in
/// applications, as well, although the predefined functions probably
/// cover most use cases.  This type is opaque from the point of view
/// of the Hier code.  The HierGenerator make_exporter(), push(),
/// and make_from_exporter() members provide the means for
/// constructing and using a HierExporter instance.

pub trait HierExporter {
    fn as_any    (&self)     -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Users can traverse Rustics instances in a Hier instance by
/// implementing this trait and calling one of the traverse methods,
/// traverse_live(), or traverse_all().

pub trait HierTraverser {
    /// This method is invoked on each Rustics instance in the
    /// matrix.

    fn visit(&mut self, member: &mut dyn Rustics);
}

// The HierGenerator trait defines the interface that allows a Rustics
// type to support hierarchical statistics.  This code connects the Hier
// impl code with the impl code for the underlying Rustics type.
//
// HierGenerator is thus an abstraction of the associated functions
// that are not members (in the impl code, but not taking &self as
// a parameter).
//
// The HierGenerator implementation for the RunningInteger
// type is a good example to read if you want to understand
// this code.

/// The HierGenerator trait defines the interface that allows a Rustics
/// type to support hierarchical statistics.  This code connects the Hier
/// impl code with the impl code for the underlying Rustics type.  It is
/// used only to add interfaces for types, so users will need it only if
/// they implement a custom Rustics implementation and wish to support
/// Hier instances based on that type.

pub trait HierGenerator {
    fn make_from_exporter(&self, name: &str, print_opts: &PrintOption, exports: ExporterRc)
            -> MemberRc;

    fn make_window(&self, name: &str, window_size: usize, print_opts: &PrintOption)
            -> Box<dyn Rustics>;

    fn make_member   (&self, name: &str, print_opts: &PrintOption) -> MemberRc;
    fn make_exporter (&self) -> ExporterRc;
    fn push          (&self, exports: &mut dyn HierExporter, member: MemberRc);
    fn hz            (&self) -> u128;
}

// The HierMember trait is used to extend a specific type
// implementing the Rustics trait to work with the Hier code.
//
// The code for the Hier type and the HierGenerator just need to
// be able to upcast and downcast into the member types.

/// The HierMember trait extends a Rustics implementation to interface
/// with the Hier code.  This trait is of use only if implementing a
/// custom Rustics type.

pub trait HierMember {
    fn to_rustics    (&self    ) -> &dyn Rustics;
    fn to_rustics_mut(&mut self) -> &mut dyn Rustics;
    fn to_histogram  (&    self) -> &dyn Histogram;
    fn as_any        (&self    ) -> &dyn Any;
    fn as_any_mut    (&mut self) -> &mut dyn Any;
}

// The Hier type implements a type of hierarchical statistics
// collector using a HierGenerator instance and HierMember instances.

/// Hier instances are the concrete type for a statistics
/// hierarchy.
///
/// See the module comments for sample code.

pub struct Hier {
    dimensions:     Vec<HierDimension>,
    generator:      GeneratorRc,
    stats:          Vec<Window<MemberRc>>,
    name:           String,
    title:          String,
    id:             usize,
    class:          String,
    auto_next:      i64,
    advance_count:  i64,
    event_count:    i64,
    printer:        PrinterBox,
    print_opts:     PrintOption,
    window:         Option<Box<dyn Rustics>>,
}

/// HierConfig defines the configuration parameters for a Hier
/// instance.  Most users should use the prepackaged Hier constructors
/// like IntegerHier::new_hier and TimeHier::new_hier.

pub struct HierConfig {
    pub name:        String,
    pub descriptor:  HierDescriptor,
    pub generator:   GeneratorRc,
    pub window_size: Option<usize>,
    pub class:       String,
    pub print_opts:  PrintOption,
}

impl Hier {
    /// The new() function creates a hier instance. It generally
    /// should be called from the constructor for the specific type
    /// for the hierarchy.  For example, the IntegerHier impl provides
    /// the constructor new_hier() that will invoke this function
    /// to create a RunningInteger hierarchy.

    pub fn new(configuration: HierConfig) -> Hier {
        let     descriptor    = configuration.descriptor;
        let     generator     = configuration.generator;
        let     name          = configuration.name;
        let     class         = configuration.class;
        let     print_opts    = configuration.print_opts;

        let     auto_next     = descriptor.auto_next;
        let     dimensions    = descriptor.dimensions;
        let     id            = usize::MAX;
        let     advance_count = 0;
        let     event_count   = 0;
        let mut stats         = Vec::with_capacity(dimensions.len());

        if dimensions.is_empty() {
            panic!("Hier::new:  No dimensions were specified.");
        }

        for i in 0..dimensions.len() - 1 {
            if dimensions[i].period < 2 {
                panic!("Hier::new:  The period must be at least 2.");
            }
        }

        let window_size =
            if let Some(window_size) = &configuration.window_size {
                *window_size
            } else {
                0
            };

        let (printer, title, _units, _histo_opts) = parse_print_opts(&print_opts, &name);

        let window =
            if window_size > 0 {
                let generator = generator.borrow();
                let window    = generator.make_window(&name, window_size, &print_opts);

                Some(window)
            } else {
                None
            };

        // Create the set of windows that we use to hold all
        // the actual Rustics instances.

        for dimension in &dimensions {
            stats.push(Window::new(dimension.retention, dimension.period));
        }

        // Make the first Rustics instance so that we are ready to record data.

        let member = generator.borrow_mut().make_member(&name, &print_opts);

        assert!(hier_item!(member).to_rustics().class() == class);

        stats[0].push(member);

        Hier {
            dimensions,   generator,   stats,
            name,         title,       id,
            class,        auto_next,   advance_count,
            event_count,  printer,     print_opts,
            window
        }
    }

    /// The current() method returns the newest Rustics instance
    /// at the lowest level, which is the only Rustics instance
    /// that records data.  The other members are read-only.

    pub fn current(&self) -> MemberRc {
        let member = self.stats[0].newest().unwrap();

        member.clone()
    }

    /// Prints the given instance.

    pub fn print_index_opts(&self, index: HierIndex, printer: PrinterOption, title: Option<&str>) {
        self.local_print(index, printer, title);
    }

    /// Prints every member of the Hier instance.

    pub fn print_all(&self, printer: PrinterOption, title: Option<&str>) {
        for i in 0..self.stats.len() {
            let level = &self.stats[i];

            for j in 0..level.all_len() {
                let index = HierIndex::new(HierSet::All, i, j);

                self.local_print(index, printer.clone(), title)
            }
        }
    }

    /// Deletes all the Rustics instances from the windows, as well as any
    /// related data.  After clearing the struct, it pushes a new level 0
    /// Rustics instance to receive data.
    ///
    /// This operation sets the instance back to its initial state.

    pub fn clear_all(&mut self) {
        self.advance_count = 0;
        self.event_count   = 0;

        // Clear all the windows.

        for level in &mut self.stats {
            level.clear();
        }

        // Create the first level 0 instance.

        let generator = self.generator.borrow_mut();
        let member    = generator.make_member(&self.name, &self.print_opts);

        // Push this level 0 instance into the window.

        self.stats[0].push(member);

        // Clear the Rustics instance collecting the most recent
        // samples, if configured.

        if let Some(window) = &mut self.window {
            window.clear();
        }
    }

    /// Invokes a user-supplied functions on every member of the set hierarchy.

    pub fn traverse_all(&mut self, traverser: &mut dyn HierTraverser) {
        for level in &mut self.stats {
            for member in level.iter_all() {
                let borrow  = hier_item_mut!(member);
                let rustics = borrow.to_rustics_mut();

                traverser.visit(rustics);
            }
        }
    }

    /// Invokes a user-supplied function on all the live members on every level.

    pub fn traverse_live(&mut self, traverser: &mut dyn HierTraverser) {
        for level in &mut self.stats {
            for member in level.iter_live() {
                let borrow  = hier_item_mut!(member);
                let rustics = borrow.to_rustics_mut();

                traverser.visit(rustics);
            }
        }
    }

    /// Returns the member at the given index, if such exists.

    pub fn index(&self, index: HierIndex) -> Option<MemberRc> {
        let level = index.level;
        let which = index.which;

        if level >= self.stats.len() {
            return None;
        }

        let target =
            match index.set {
                HierSet::Live => { self.stats[level].index_live(which) }
                HierSet::All  => { self.stats[level].index_all (which) }
            }?;

        Some(target.clone())
    }

    /// The sum() method allows the user to sum an arbitrary list of
    /// members of the hierarchy into a new Rustics instance. The
    /// result is not maintained in the hierarchy.

    pub fn sum(&self, addends: Vec<HierIndex>, name: &str) -> (Option<MemberRc>, usize) {
        let     generator    = self.generator.borrow();
        let     exporter_rc  = generator.make_exporter();
        let     mut exporter = exporter_rc.borrow_mut();
        let mut misses       = 0;

        // Gather a list of the members to sum.

        for index in &addends {
            match self.index(*index) {
                Some(member) => { generator.push(&mut *exporter, member); }
                None         => { misses += 1; }
            }
        }

        let valid = addends.len() - misses;

        if valid == 0 {
            return (None, 0)
        }

        drop(exporter);

        // Now make the sum Rustics instance.

        let sum = generator.make_from_exporter(name, &self.print_opts, exporter_rc);

        (Some(sum), valid)
    }

    /// The advance() method pushes a new level 0 Rustics instance into
    /// the level 0 window.  It also updates the upper levels as needed.
    /// The user can call this directly or use auto_advance.  The code
    /// doesn't prevent mixing the two, but the results will be odd if both
    /// are used.

    pub fn advance(&mut self) {
        // Increment the advance op count.  This counts the number
        // of level 0 Rustics instances pushed, and thus tells
        // us when we need to push a new higher-level instance.

        self.advance_count += 1;

        // Now move up the stack.

        let mut advance_point = 1;
        let     generator     = self.generator.borrow();

        // Check whether we have enough new instances at a given level
        // to push a new sum of statistics to a higher level.

        for i in 0..self.dimensions.len() - 1 {
            advance_point *= self.dimensions[i].period as i64;

            if self.advance_count % advance_point == 0 {
                let exporter = self.make_and_fill_exporter(i);
                let name     = &self.name;
                let new_stat = generator.make_from_exporter(name, &self.print_opts, exporter);

                self.stats[i + 1].push(new_stat);
            } else {
                break;
            }
        }

        // Create the new Rustics instance to collect data and push it into
        // the level zero window.

        let member = generator.make_member(&self.name, &self.print_opts);

        self.stats[0].push(member);
    }

    /// Returns the number of live members at the given level.

    pub fn live_len(&self, level: usize) -> usize {
        self.stats[level].live_len()
    }

    /// Returns the count of all the members at the given level.

    pub fn all_len(&self, level: usize) -> usize {
        self.stats[level].all_len()
    }

    /// Returns the total number of statistics samples recorded into
    /// the Hier instance since its creation or since the the last
    /// clear_all invocation.

    pub fn event_count(&self) -> i64 {
        self.event_count
    }

    pub fn hz(&self) -> u128 {
        let generator = self.generator.borrow();

        generator.hz()
    }

    // Prints one Rustics instance using the Rustics trait.  This method
    // always appends the indices to the title.

    fn local_print(&self, index: HierIndex, printer_opt: PrinterOption, title_opt: Option<&str>) {
        let level = index.level;
        let which = index.which;

        let title =
            if let Some(title) = title_opt {
                title
            } else {
                &self.title
            };

        let set =
            match index.set {
                HierSet::Live => { "live" }
                HierSet::All  => { "all"  }
            };

        let title = format!("{}[{}].{}[{}]", title, level, set, which);

        let printer_box =
            if let Some(printer) = printer_opt.clone() {
                printer
            } else {
                self.printer.clone()
            };

        if level >= self.stats.len() {
            let printer = printer_mut!(printer_box);
            printer.print(&title);
            printer.print(&format!("  That level ({}) is invalid ({} levels configured).",
                level, self.stats.len()));
            return;
        }

        let target = self.index(index);

        let target =
            if let Some(target) = target {
                target
            } else {
                let printer = printer_mut!(printer_box);

                printer.print(&title);
                printer.print(&format!("  That index ({}) is out of bounds.", which));
                return;
            };

        // Downcast to the Rustics trait and print.

        let target = target.borrow();
        target.to_rustics().print_opts(printer_opt, title_opt);
    }

    // Use an exporter to create a list of Rustics instances
    // to be summed to create a higher-level instance.

    fn make_and_fill_exporter(&self, level: usize) -> ExporterRc {
        let     generator    = self.generator.borrow();
        let     exporter_rc  = generator.make_exporter();
        let mut exporter     = exporter_rc.borrow_mut();

        let level = &self.stats[level];

        // Gather the list of Rustics instances to sum into a new
        // member.

        for stat in level.iter_live() {
            generator.push(&mut *exporter, stat.clone());
        }

        drop(exporter);
        exporter_rc
    }

    // Check the event count to see whether it's time to push a new
    // level 0 instance.  This method implements the auto_next
    // feature.

    fn check_and_advance(&mut self) {
        // Push a new instance if we've reached the event limit
        // for the current one.  Do this before we push the next
        // event so that users see an empty current Rustics instance
        // only before recording any events at all.

        if
            self.auto_next != 0
        &&  self.event_count > 0
        &&  self.event_count % self.auto_next == 0 {
            self.advance();
        }

        // Advance the event count.  This method should only be
        // called when a statistical value is being recorded.

        self.event_count += 1;
    }

    fn title_all(&mut self) {
        let mut traverser = TitleAll::new(&self.title);

        self.traverse_all(&mut traverser);

        if let Some(window) = &mut self.window {
            window.set_title(&self.title);
        }
    }
}

// Implement the Rustics trait for the Hier instance.  Unless
// a window has been configured, the Rustics code returns data
// from the newest level 0 instance, which is the only one
// receiving data.

impl Rustics for Hier {
    fn record_i64(&mut self, value: i64) {
        self.check_and_advance();

        let member  = self.stats[0].newest_mut().unwrap();
        let borrow  = hier_item_mut!(member);
        let rustics = borrow.to_rustics_mut();

        rustics.record_i64(value);

        if let Some(window) = &mut self.window {
            window.record_i64(value);
        }
    }

    fn record_f64(&mut self, sample: f64) {
        self.check_and_advance();

        let current = self.current();
        let borrow  = hier_item_mut!(current);
        let rustics = borrow.to_rustics_mut();

        rustics.record_f64(sample);

        if let Some(window) = &mut self.window {
            window.record_f64(sample);
        }
    }

    fn record_event(&mut self) {
        let _ = self.record_event_report();
    }

    fn record_event_report(&mut self) -> i64 {
        self.check_and_advance();

        let member  = self.stats[0].newest_mut().unwrap();
        let borrow  = hier_item_mut!(member);
        let rustics = borrow.to_rustics_mut();

        // Now record the event in the window, if one is present.

        let sample = rustics.record_event_report();

        if let Some(window) = &mut self.window {
            assert!(self.class == "time");
            window.record_time(sample);
        }

        sample
    }

    fn record_time(&mut self, sample: i64) {
        self.check_and_advance();

        let current = self.current();
        let borrow  = hier_item_mut!(current);
        let rustics = borrow.to_rustics_mut();

        rustics.record_time(sample);

        if let Some(window) = &mut self.window {
            window.record_time(sample);
        }
    }

    fn record_interval(&mut self, timer: &mut TimerBox) {
        self.check_and_advance();

        let current = self.current();
        let borrow  = hier_item_mut!(current);
        let rustics = borrow.to_rustics_mut();
        let time    = timer_mut!(timer).finish();

        rustics.record_time(time);

        if let Some(window) = &mut self.window {
            window.record_time(time);
        }
    }

    // We return the name and title of the Hier instance itself.

    fn name(&self) -> String {
        self.name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn class(&self) -> &str {
        &self.class
    }

    fn count(&self) -> u64 {
        if let Some(window) = &self.window {
            window.count()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.count()
        }
    }

    fn log_mode(&self) -> isize {
        if let Some(window) = &self.window {
            window.log_mode()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.log_mode()
        }
    }

    fn mean(&self) -> f64 {
        if let Some(window) = &self.window {
            window.mean()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.mean()
        }
    }

    fn standard_deviation(&self) -> f64 {
        if let Some(window) = &self.window {
            window.standard_deviation()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.standard_deviation()
        }
    }

    fn variance(&self) -> f64 {
        if let Some(window) = &self.window {
            window.variance()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.variance()
        }
    }

    fn skewness(&self) -> f64 {
        if let Some(window) = &self.window {
            window.skewness()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.skewness()
        }
    }

    fn kurtosis(&self) -> f64 {
        if let Some(window) = &self.window {
            window.kurtosis()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.kurtosis()
        }
    }

    fn int_extremes(&self) -> bool {
        if let Some(window) = &self.window {
            window.int_extremes()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.int_extremes()
        }
    }

    fn float_extremes(&self) -> bool {
        if let Some(window) = &self.window {
            window.float_extremes()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.float_extremes()
        }
    }

    fn min_i64(&self) -> i64  {
        if let Some(window) = &self.window {
            window.min_i64()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.min_i64()
        }
    }

    fn min_f64(&self) -> f64 {
        if let Some(window) = &self.window {
            window.min_f64()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.min_f64()
        }
    }

    fn max_i64(&self) -> i64 {
        if let Some(window) = &self.window {
            window.max_i64()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.max_i64()
        }
    }

    fn max_f64(&self) -> f64 {
        if let Some(window) = &self.window {
            window.max_f64()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.max_f64()
        }
    }

    fn precompute(&mut self) {
        if let Some(window) = &mut self.window {
            window.precompute();
        } else {
            let current = self.current();
            let borrow  = hier_item_mut!(current);
            let rustics = borrow.to_rustics_mut();

            rustics.precompute();
        }
    }

    /// Deletes all the statistical data in the Hier object.

    fn clear(&mut self) {
        self.clear_all();
    }

    // Functions for printing

    fn print(&self) {
        if let Some(window) = &self.window {
            window.print_opts(None, Some(&self.title));
        } else {
            let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

            self.local_print(index, None, None);
        }
    }

    fn print_opts(&self, printer: PrinterOption, title: Option<&str>) {
        let title =
            if let Some(title) = title {
                title
            } else {
                &self.title
            };

        if let Some(window) = &self.window {
            window.print_opts(printer, Some(title));
        } else {
            let index = HierIndex::new(HierSet::Live, 0, self.live_len(0) - 1);

            self.local_print(index, printer, Some(title));
        }
    }

    fn log_histogram(&self) -> Option<LogHistogramBox> {
        if let Some(window) = &self.window {
            window.log_histogram()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.log_histogram()
        }
    }

    fn float_histogram(&self) -> Option<FloatHistogramBox> {
        if let Some(window) = &self.window {
            window.float_histogram()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.float_histogram()
        }
    }

    // The title is kept in the Hier instance.

    /// Sets the title used when printing.  The Hier implementation always
    /// appends the set indices to the title when printing a member.

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();

        self.title_all();
    }

    // For internal use.

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn id(&self) -> usize {
        self.id
    }

    fn equals(&self, other: &dyn Rustics) -> bool {
        if let Some(other) = <dyn Any>::downcast_ref::<Hier>(other.generic()) {
            std::ptr::eq(self, other)
        } else {
            false
        }
    }

    fn generic(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn export_stats(&self) -> ExportStats {
        if let Some(window) = &self.window {
            window.export_stats()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.export_stats()
        }
    }
}

struct TitleAll {
    title:  String,
}

impl TitleAll {
    fn new(title: &str) -> TitleAll {
        let title = title.to_string();

        TitleAll { title }
    }
}

impl HierTraverser for TitleAll {
    fn visit(&mut self, member: &mut dyn Rustics) {
        member.set_title(&self.title);
    }
}

impl Histogram for Hier {
    fn print_histogram(&self, printer: &mut dyn Printer) {
        if let Some(log_histogram) = self.log_histogram() {
            log_histogram.borrow().print(printer);
        } else if let Some(float_histogram) = self.float_histogram() {
            float_histogram.borrow().print(printer);
        }
    }

    fn clear_histogram(&mut self) {
        if let Some(log_histogram) = self.log_histogram() {
            log_histogram.borrow_mut().clear();
        } else if let Some(float_histogram) = self.float_histogram() {
            float_histogram.borrow_mut().clear();
        }
    }

    fn to_log_histogram(&self) -> Option<LogHistogramBox> {
        if let Some(window) = &self.window {
            window.log_histogram()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.log_histogram()
        }
    }

    fn to_float_histogram(&self) -> Option<FloatHistogramBox> {
        if let Some(window) = &self.window {
            window.float_histogram()
        } else {
            let current = self.current();
            let borrow  = current.borrow();
            let rustics = borrow.to_rustics();

            rustics.float_histogram()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::run_histogram_tests;
    use crate::integer_hier::IntegerHier;
    use crate::integer_hier::IntegerHierConfig;
    use crate::running_integer::RunningInteger;
    use crate::time_hier::TimeHier;
    use crate::time_hier::TimeHierConfig;
    use crate::running_time::RunningTime;
    use crate::stdout_printer;
    use crate::integer_hier::tests::make_test_hier;
    use crate::tests::continuing_box;
    use crate::tests::check_printer_box;
    use crate::tests::check_printer_count_match;

    // Make a Hier instance for testing.  The tests use the RunningInteger
    // implementation via IntegerHier.
    //
    // Make a 4-level hierarchical statistics struct for testing.

    pub fn make_hier(level_0_period: usize, auto_next: usize) -> Hier {
        let     levels      = 4;
        let     dimension   = HierDimension::new(level_0_period, 3 * level_0_period);
        let mut dimensions  = Vec::<HierDimension>::with_capacity(levels);

        // Push the level 0 descriptor.

        dimensions.push(dimension);

        // Create a hierarchy.  Use a period of 4 level at level 1,
        // and just add 2 as we go up.  This will keep all the
        // periods distinct.  We force the total window size to
        // 3 times the period, which is fine for testing.  That
        // might become too costly in actual usage.

        let mut period = 4;

        for _i in 1..levels {
            let dimension = HierDimension::new(period, 3 * period);

            dimensions.push(dimension);

            period += 2;
        }

        // Finish creating the Hier description instance.  This
        // just describes the windows and when the Rustics
        // instances are created and pushed.

        let auto_next  = Some(auto_next as i64);
        let descriptor = HierDescriptor::new(dimensions, auto_next);

        // Now create the RunningInteger-based Hier instance via
        // IntegerHier, which does some of the work for
        // us.

        let name        = "Hier Test Instance".to_string();
        let print_opts  = None;
        let window_size = None;

        // Finally, create the configuration description for the
        // constructor.

        let configuration =
            IntegerHierConfig { descriptor, name, window_size, print_opts };

        // Make the actual Hier instance.  new_hier() handles the
        // parameters specific for using RunningInteger instances.

        IntegerHier::new_hier(configuration)
    }

    fn compute_events_per_entry(hier_integer: &Hier, level: usize) -> i64 {
        let mut result = hier_integer.auto_next as i64;

        assert!(result > 0);

        for i in 0..level {
            result *= hier_integer.dimensions[i].period as i64;
        }

        result
    }

    fn roundup(value: i64, multiple: i64) -> i64 {
        (((value + multiple - 1) / multiple)) * multiple
    }

    fn compute_len(hier_integer: &Hier, level: usize, set: HierSet, events: i64) -> usize {
        assert!(events > 0 || level == 0);

        let recorded_events =
            if level == 0 {
                let auto_next = hier_integer.auto_next as i64;

                roundup(events, auto_next)
            } else {
                events - 1
            };

        let     events_per_entry = compute_events_per_entry(&hier_integer, level);
        let     pushes           = recorded_events / events_per_entry;
        let     period           = hier_integer.dimensions[level].period as i64;
        let     size_limit       = hier_integer.dimensions[level].retention as i64;

        let mut length =
            match set {
                HierSet::Live => { std::cmp::min(pushes, period    ) }
                HierSet::All  => { std::cmp::min(pushes, size_limit) }
            };

        if length == 0 && level == 0 {
            length = 1;
        }

        length as usize
    }

    fn check_sizes(hier_integer: &Hier, events: i64, verbose: bool) {
        for level in 0..hier_integer.stats.len() {
            let expected_all_len  = compute_len(hier_integer, level, HierSet::All,  events);
            let expected_live_len = compute_len(hier_integer, level, HierSet::Live, events);

            let actual_all_len    = hier_integer.stats[level].all_len();
            let actual_live_len   = hier_integer.stats[level].live_len();

            if verbose {
                println!("check_sizes:  at level {}, events {}", level, events);

                println!("    live {} == {}",
                    actual_live_len, expected_live_len);

                println!("    all {} == {}",
                    actual_all_len, expected_all_len);
            }

            assert!(actual_all_len  == expected_all_len );
            assert!(actual_live_len == expected_live_len);
        }
    }

    // This is a fairly straightforward test that just pushes a lot of
    // values into a Hier instance.  It is long because it takes a fair
    // number of operations to force higher-level instances into existence.

    fn simple_hier_test() {
        let     auto_next      = 4;
        let     level_0_period = 4;
        let     signed_auto    = auto_next as i64;
        let mut events         = 0;
        let mut sum_of_events  = 0;
        let mut hier_integer   = make_hier(level_0_period, auto_next);
        let     hier_integer_2 = make_hier(level_0_period, auto_next);

        let expected_len = compute_len(&hier_integer, 0, HierSet::All,  events);
        assert!(expected_len == 1);

        // Do a quick sanity test on equals().

        assert!( hier_integer.equals(&hier_integer));
        assert!(!hier_integer.equals(&hier_integer_2));

        // Check that the instance matches our expectations.

        let live_len  = hier_integer.stats[0].live_len();
        let all_len   = hier_integer.stats[0].all_len();
        let period    = hier_integer.dimensions[0].period;

        assert!(signed_auto == hier_integer.auto_next);
        assert!(all_len     == 1                     );
        assert!(live_len    == 1                     );
        assert!(period      == level_0_period        );

        let expected_count = auto_next as u64;

        // Push one full window and see whether the data and
        // structure matches what we expect as we record each
        // event.

        for i in 0..signed_auto {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            if i < signed_auto - 1 {
                let mean  = sum_of_events as f64 / (i + 1) as f64;

                let live_len_0 = hier_integer.stats[0].live_len();
                let all_len_0  = hier_integer.stats[0].all_len();
                let count      = (i + 1) as u64;

                assert!(all_len_0              == 1    );
                assert!(live_len_0             == 1    );
                assert!(hier_integer.count()   == count);
                assert!(hier_integer.min_i64() == 0    );
                assert!(hier_integer.max_i64() == i    );
                assert!(hier_integer.mean()    == mean );

                check_sizes(&hier_integer, events, false);
            }
        }

        let mean = sum_of_events as f64 / events as f64;

        assert!(hier_integer.count() as i64 == signed_auto    );
        assert!(hier_integer.min_i64()      == 0              );
        assert!(hier_integer.max_i64()      == signed_auto - 1);
        assert!(hier_integer.mean()         == mean           );

        check_sizes(&hier_integer, events, true);
        hier_integer.print();

        assert!(hier_integer.count() == events as u64);

        let mut sum = 0;

        for i in 0..signed_auto {
            let value = signed_auto + i;

            hier_integer.record_i64(value);
            sum           += value;
            events        += 1;
            sum_of_events += value;

            check_sizes(&hier_integer, events, false);
        }

        assert!(hier_integer.stats[0].all_len() == 2);
        hier_integer.print();

        let floating_window = signed_auto as f64;
        let expected_mean   = (sum as f64) / floating_window;

        assert!(hier_integer.count()   == expected_count     );
        assert!(hier_integer.min_i64() == signed_auto        );
        assert!(hier_integer.max_i64() == 2 * signed_auto - 1);
        assert!(hier_integer.mean()    == expected_mean      );

        // Record two windows worth of events and check that our
        // size expectations are correct.  Also, keep the sum of
        // the last n events, where n is the window size, so that
        // we can check the mean.  Only the last n events should
        // be used to compute the mean.

        let mut sum = 0;

        for i in 0..2 * signed_auto {
            let value = -i;

            hier_integer.record_i64(value);

            if i >= signed_auto {
                sum += value;
            }

            events        += 1;
            sum_of_events += value;

            check_sizes(&hier_integer, events, false);
        }

        hier_integer.print();

        let expected_mean = sum as f64 / floating_window;

        assert!(hier_integer.count()   == expected_count        );
        assert!(hier_integer.min_i64() == -(2 * signed_auto - 1));
        assert!(hier_integer.max_i64() == -signed_auto          );
        assert!(hier_integer.mean()    == expected_mean         );

        // Now force a level 1 stat instance.

        for i in 0..(auto_next * level_0_period) as i64 {
            hier_integer.record_i64(i);
            hier_integer.record_i64(-i);

            events += 2;

            check_sizes(&hier_integer, events, false);
        }

        // Check that we have at least one level 1 instance.  This
        // is a test of the test itself, for the most part.

        let expected_len = compute_len(&hier_integer, 1, HierSet::All,  events);
        let actual_len   = hier_integer.stats[1].all_len();

        assert!(expected_len > 0);
        assert!(actual_len == expected_len);

        // Now force a level 2 instance.

        for i in 0..(auto_next * level_0_period / 2) as i64 {
            hier_integer.record_i64(i);
            events        += 1;
            sum_of_events += i;

            check_sizes(&hier_integer, events, false);
        }

        for i in 0..(auto_next * level_0_period / 2) as i64 {
            hier_integer.record_i64(i);

            events        += 1;
            sum_of_events += i;

            check_sizes(&hier_integer, events, false);
        }

        // Compute the expected mean once we force level 0 to
        // be summed.

        let expected_mean = sum_of_events as f64 / events as f64;

        // Force the next push from level 0 by recording an event.
        // This should produce a level 2 entry.

        let value = 0;

        hier_integer.record_i64(value);

        events        += 1;
        sum_of_events += value;

        let expected_len = compute_len(&hier_integer, 2, HierSet::All, events);

        assert!(expected_len == 1);

        // Check the length.  Use a hardcode value, too, to check the
        // sanity of the preceding code.

        assert!(hier_integer.stats[2].all_len() == expected_len);

        let stat_rc      = hier_integer.stats[2].newest().unwrap();
        let stat_borrow  = stat_rc.borrow();
        let stat         = stat_borrow.to_rustics();

        // Do a sanity test on equals.

        assert!(!hier_integer.equals(stat));

        stat.print();

        hier_integer.print_all(None, None);

        assert!(stat.mean() == expected_mean);

        println!("simple_hier_test:  {} events, sum {}", events, sum_of_events);
    }

    struct TestTraverser {
        count:  i64,
    }

    impl TestTraverser {
        pub fn new() -> TestTraverser {
            let count = 0;

            TestTraverser { count }
        }
    }

    impl HierTraverser for TestTraverser {
        fn visit(&mut self, _member: &mut dyn Rustics) {
            self.count += 1;
        }
    }

    // Shove enough events into the stat to get a level 3 entry.  Check that
    // the count and the mean of the level 3 stat match our expectations.

    fn long_test() {
        let     auto_next      = 2;
        let     level_0_period = 4;
        let mut hier_integer   = make_hier(level_0_period, auto_next);
        let mut events         = 0;

        // Record data until there's a level 3 instance.

        while hier_integer.stats[3].all_len()  == 0 {
            events += 1;
            hier_integer.record_i64(events);

            check_sizes(&hier_integer, events, false);
        }

        // Now see that all the sizes, etc, match what we
        // expect.

        let     dimensions         = &hier_integer.dimensions;
        let mut events_per_level_3 = auto_next;

        for i in 0..dimensions.len() - 1 {
            events_per_level_3 *= dimensions[i].period;
        }

        {
            let stat_rc        = hier_integer.stats[3].newest().unwrap();
            let stat_borrow    = stat_rc.borrow();
            let stat           = stat_borrow.to_rustics();

            let events_in_stat = (events - 1) as f64;
            let sum            = (events_in_stat * (events_in_stat + 1.0)) / 2.0;
            let mean           = sum / events_in_stat;

            println!("long_test:  stats.mean() {}, expected {}", stat.mean(), mean);

            assert!(stat.count() as i64 == events - 1               );
            assert!(stat.count() as i64 == events_per_level_3 as i64);
            assert!(stat.mean()         == mean                     );

            hier_integer.print_all(None, None);
        }

        // Do a quick test of traverse_live().  It should see each
        // "live" Rustics instance in the matrix.

        let mut traverser = TestTraverser::new();

        hier_integer.traverse_live(&mut traverser);

        // Now compute how many members the traverser should have seen.

        let mut predicted = 0;

        for level in 0..hier_integer.dimensions.len() {
            predicted += hier_integer.live_len(level) as i64;
        }

        println!("long_test:  traverse_live saw {} stats instances, predicted {}",
            traverser.count, predicted);
        assert!(traverser.count == predicted);

        // Do a quick test of traverse_all().  It should see each
        // instance in the matrix.

        let mut traverser = TestTraverser::new();

        hier_integer.traverse_all(&mut traverser);

        // Now compute how many members the traverser should have seen.

        let mut predicted = 0;

        for level in 0..hier_integer.dimensions.len() {
            predicted += hier_integer.all_len(level) as i64;
        }

        println!("long_test:  traverse_all saw {} stats instances, predicted {}",
            traverser.count, predicted);
        assert!(traverser.count == predicted);

        // Do a sanity test on the members.

        let member_opt    = hier_integer.stats[1].newest().unwrap();
        let member_borrow = hier_item!(member_opt);
        let member        = member_borrow.as_any().downcast_ref::<RunningInteger>();

        // For test debugging.
        //
        //let proper_type =
        //    match member {
        //        Some(_) => { true  }
        //        None    => { false }
        //    };
        //
        //assert!(proper_type);

        let rustics = member.unwrap().to_rustics();

        println!("long_test:  got \"{}\" for class", rustics.class());

        assert!(rustics.class() == "integer");
    }

    fn test_sanity() {
        let printer      = stdout_printer();
        let printer      = printer_mut!(printer);
        let name         = "time_hier sanity test".to_string();
        let timer        = continuing_box();
        let print_opts   = None;

        // Create the dimensions.

        let dimension_0  = HierDimension::new(4, 4);
        let dimension_1  = HierDimension::new(100, 200);

        let dimensions   = vec![ dimension_0.clone(), dimension_1.clone() ];

        let auto_next    = 20;
        let auto_advance = Some(auto_next);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);
        let window_size  = None;

        // Now make an actual time_hier instance from a configuration.

        let configuration =
            TimeHierConfig { descriptor, timer, name, window_size, print_opts };

        let mut hier = TimeHier::new_hier(configuration);

        // Now force a level 1 instance.

        let mut events             = 0;
        let     events_per_level_1 = auto_next * dimension_0.period() as i64;
        let mut timer              = continuing_box();

        for i in 0..2 * events_per_level_1 {
            hier.record_time(i + 1);
            hier.record_interval(&mut timer);

            events += 2;
        }

        assert!(hier.event_count() == events);
        hier.print();
        hier.print_histogram(printer);

        hier.clear_histogram();

        let histogram = hier.to_log_histogram().unwrap();
        let histogram = histogram.borrow();

        for sample in histogram.positive.iter() {
            assert!(*sample == 0);
        }

        // Do a sanity test on the members.

        let member_opt    = hier.stats[1].newest().unwrap();
        let member_borrow = hier_item!(member_opt);
        let member        = member_borrow.as_any().downcast_ref::<RunningTime>();

        // For test debugging.
        //
        //let proper_type =
        //    match member {
        //        Some(_) => { true  }
        //        None    => { false }
        //    };
        //
        //assert!(proper_type);

        let rustics = member.unwrap().to_rustics();

        assert!(rustics.count() == events_per_level_1 as u64);
        assert!(rustics.class() == "time");

        println!("test_sanity:  got \"{}\" for class", rustics.class());

        let index   = HierIndex::new(HierSet::Live, 0, 0);
        let printer = stdout_printer();
        let title   = "Index Print Title";

        hier.print_index_opts(index, Some(printer), Some(title));

        let export = hier.export_stats();

        assert!(export.printable.n as i64 == auto_next);
    }

    fn sample_usage() {
        // Make a descriptor of the first level.  We have chosen to sum 1000
        // level 0 RunningInteger instances into one level 1 RunningInteger
        // instance.  This level is large, so we will keep only 1000 level 0
        // instances in the window.

        let dimension_0 = HierDimension::new(1000, 1000);

        // At level 1, we want to sum 100 level 1 instances into one level 2
        // instance.  This level is smaller, so let's retain 200
        // RunningInteger instances here.

        let dimension_1 = HierDimension::new(100, 200);

        // Level two isn't summed, so the period isn't used.  Let's pretend
        // this level isn't used much, so retain only 100 instances in it.

        let dimension_2 = HierDimension::new(0, 100);

        // Now create the Vec.  Save the dimension instances for future use.

        let dimensions =
            vec![
                dimension_0.clone(), dimension_1.clone(), dimension_2.clone()
            ];

        // Now create the entire descriptor for the hier instance.  Let's
        // record 2000 events into each level 0 RunningInteger instance.

        let auto_advance = Some(2000);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);

        // Now create some items used by Hier to do printing.

        let name        = "test hierarchical integer".to_string();
        let print_opts  = None;
        let window_size = None;

        // Finally, create the configuration description for the
        // constructor.

        let configuration =
            IntegerHierConfig { descriptor, name, window_size, print_opts };

        // Now make the Hier instance.

        let mut integer_hier = IntegerHier::new_hier(configuration);

        // Now record some events with boring data.

        let mut events   = 0;
        let auto_advance = auto_advance.unwrap();

        for i in  0..auto_advance {
            events += 1;
            integer_hier.record_i64(i + 10);
        }

        assert!( integer_hier.log_mode() == 11);
        assert!( integer_hier.int_extremes());
        assert!(!integer_hier.float_extremes());

        // This is a no-op that shouldn't panic.

        integer_hier.precompute();

        // We have just completed the first level 0 instance, but
        // the implementation creates the next instance only when
        // it has data to record, so there should be only one level
        // zero instance, and nothing at level 1 or level 2.

        assert!(integer_hier.event_count() == events);
        assert!(integer_hier.count()       == events as u64);
        assert!(integer_hier.live_len(0)   == 1     );
        assert!(integer_hier.live_len(1)   == 0     );
        assert!(integer_hier.live_len(2)   == 0     );

        // Now record some data to force the creation of
        // the second level 1 instance.

        events += 1;
        integer_hier.record_i64(10);

        // The new level 0 instance should have only one event
        // recorded.  The Rustics implementation for Hier returns
        // the data in the current level 0 instance, so check it.

        assert!(integer_hier.event_count() == events);
        assert!(integer_hier.count()       == 1     );
        assert!(integer_hier.live_len(0)   == 2     );
        assert!(integer_hier.live_len(1)   == 0     );
        assert!(integer_hier.live_len(2)   == 0     );

        let events_per_level_1 =
            auto_advance * dimension_0.period() as i64;

        for i in events..events_per_level_1 {
            integer_hier.record_i64(i);
            events += 1;
        }

        // Check the state again.  We need to record one more
        // event to cause the summation at level 0 into level
        // 1.

        let expected_live  = dimension_0.period();
        let expected_count = auto_advance as u64;

        assert!(integer_hier.event_count() == events        );
        assert!(integer_hier.count()       == expected_count);
        assert!(integer_hier.live_len(0)   == expected_live );
        assert!(integer_hier.live_len(1)   == 0             );
        assert!(integer_hier.live_len(2)   == 0             );

        integer_hier.record_i64(42);
        events += 1;

        assert!(integer_hier.live_len(1)   == 1     );
        assert!(integer_hier.event_count() == events);

        // Test the histograms while we have a Hier.

        run_histogram_tests(&mut integer_hier as &mut dyn Rustics);

        // Test clear_all with a window.

        integer_hier.clear_all();

        assert!(integer_hier.count()            == 0);
        assert!(integer_hier.event_count()      == 0);
        assert!(integer_hier.stats[0].all_len() == 1);
        assert!(integer_hier.advance_count      == 0);

        for level in 1..integer_hier.stats.len() {
            assert!(integer_hier.stats[level].all_len() == 0);
        }

        // Record some events and check sizes.

        for i in 1..=auto_advance {
            integer_hier.record_i64(i as i64);
        }

        assert!(integer_hier.count()            == auto_advance as u64);
        assert!(integer_hier.event_count()      == auto_advance as i64);
        assert!(integer_hier.stats[0].all_len() == 1);
        assert!(integer_hier.stats[1].all_len() == 0);

        // Create one more level 0 Rustics instance.

        for i in 1..=auto_advance {
            integer_hier.record_i64(i as i64);
        }

        assert!(integer_hier.count()            == auto_advance as u64);
        assert!(integer_hier.event_count()      == 2 * auto_advance as i64);
        assert!(integer_hier.stats[0].all_len() == 2);
        assert!(integer_hier.stats[1].all_len() == 0);

        // Test printing with bad indices.

        let expected =
            [
                "test hierarchical integer[5000].live[0]",
                "  That level (5000) is invalid (3 levels configured).",
                "test hierarchical integer[0].live[10000]",
                "  That index (10000) is out of bounds."
            ];

        let check_printer = check_printer_box(&expected, true, false);

        let index = HierIndex::new(HierSet::Live, 5000, 0);

        integer_hier.print_index_opts(index, Some(check_printer.clone()), None);

        let index = HierIndex::new(HierSet::Live, 0, 10000);

        integer_hier.print_index_opts(index, Some(check_printer.clone()), None);

        assert!(check_printer_count_match(check_printer));
    }

    fn test_sum() {
        let     printer      = stdout_printer();
        let     printer      = printer_mut!(printer);
        let     window_size  = 200;
        let mut integer_hier = make_test_hier(100, Some(200), None);

        for i in 1..=window_size {
            integer_hier.record_i64(i as i64);
        }

        integer_hier.print_opts(None, None);
        integer_hier.print_histogram(printer);

        let count = window_size as f64;
        let sum   = (count * (count + 1.0)) / 2.0;
        let mean  = sum / count;

        integer_hier.print();

        assert!(integer_hier.mean()     == mean);
        assert!(integer_hier.variance() >   0.0);
        assert!(integer_hier.skewness() ==  0.0);
        assert!(integer_hier.log_mode() ==  8  );
        assert!(integer_hier.kurtosis() == -1.2);

        assert!(integer_hier.standard_deviation() >  0.0);

        for i in 1..10000 {
            integer_hier.record_i64(i as i64);
        }

        // Create a vector of indices.

        let mut addends      = Vec::new();
        let     level        = 0;
        let     addend_count = 1000;

        for i in 0..addend_count {
            addends.push(HierIndex::new(HierSet::Live, level, i));
        }

        // Now compute the sum and print it.

        let (sum, count) = integer_hier.sum(addends, "Level 0 Summary");
        let sum          = sum.unwrap();

        assert!(count > 0);
        assert!(count < addend_count);
        assert!(sum.borrow().to_rustics().mean() > 0.0);

        let mut addends = Vec::new();

        for i in 0..addend_count {
            addends.push(HierIndex::new(HierSet::Live, 500, i));
        }

        let (_sum, count) = integer_hier.sum(addends, "Level 0 Summary");

        assert!(count == 0);

        integer_hier.clear_all();

        assert!(integer_hier.count() == 0);

        // Test title handling.

        let new_title   = "Hier Test Title";
        let start_title = integer_hier.title();

        assert!(start_title != new_title);

        integer_hier.set_title(new_title);

        assert!(integer_hier.title() == new_title);

        // This is a no-op that should not panic.

        integer_hier.precompute();

        // Check set_id() and id().

        integer_hier.set_id(1);
        assert!(integer_hier.id() == 1);

        integer_hier.set_id(2);
        assert!(integer_hier.id() == 2);

        let export = integer_hier.export_stats();

        assert!(export.printable.n == 0);
    }

    #[test]
    #[should_panic]
    fn test_bad_retention() {
        let _ = HierDimension::new(8, 4);
    }

    #[test]
    #[should_panic]
    fn test_bad_auto_next() {
        let _ = HierDescriptor::new(Vec::new(), Some(-2));
    }

    #[test]
    #[should_panic]
    fn test_float_histogram() {
        let hier = make_test_hier(100, None, None);
        let _    = hier.float_histogram().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_float_histogram_window() {
        let hier = make_test_hier(100, Some(100), None);
        let _    = hier.float_histogram().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_short_dimensions() {
        let dimensions   = Vec::new();
        let auto_advance = Some(2000);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);
        let name         = "Test Hier".to_string();
        let window_size  = None;
        let print_opts   = None;

        let config =
            IntegerHierConfig { descriptor, name, window_size, print_opts };

        let _hier = IntegerHier::new_hier(config);
    }

    #[test]
    #[should_panic]
    fn test_zero_period() {
        let dimension_0 = HierDimension::new(1000, 1000);
        let dimension_1 = HierDimension::new(   0,  200);
        let dimension_2 = HierDimension::new(   0,  200);

        let dimensions =
            vec![ dimension_0, dimension_1, dimension_2 ];

        let auto_advance = Some(2000);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);

        let name         = "Test Hier".to_string();
        let window_size  = None;
        let print_opts   = None;

        let config =
            IntegerHierConfig { descriptor, name, window_size, print_opts };

        let _hier = IntegerHier::new_hier(config);
    }

    #[test]
    #[should_panic]
    fn test_one_period() {
        let dimension_0  = HierDimension::new(1000, 1000);
        let dimension_1  = HierDimension::new(   1,  200);
        let dimension_2  = HierDimension::new(   0,  200);

        let dimensions =
            vec![ dimension_0, dimension_1, dimension_2 ];

        let auto_advance = Some(2000);
        let descriptor   = HierDescriptor::new(dimensions, auto_advance);

        let name         = "Test Hier".to_string();
        let window_size  = None;
        let print_opts   = None;

        let config =
            IntegerHierConfig { descriptor, name, window_size, print_opts };

        let _hier = IntegerHier::new_hier(config);
    }

    #[test]
    fn run_tests() {
        simple_hier_test();
        long_test       ();
        test_sanity     ();
        test_sum        ();
        sample_usage    ();
    }
}
