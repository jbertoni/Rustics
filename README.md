# Rustics
A minimal statistics library for performance monitoring


Rustics implements a simple set of statistics objects intended primarily for
measuring performance parameters.  It provides statistics objects that
collect data and compute various statistical characterizations of the data,
such as the mean and variance.

Many testing modules contain very simple example routines called "sample_usage".

At this time, statistics can be gathered for i64 values and for time values.
They can be collected as a running total of all samples recorded, or as a
window covering the last N events, for a programmable N.  The interface does
support a "clear" function to discard all gather statistics.  See the time.rs
file for information on timers and to see some sample implementations.

In addition to the more detailed statistics type, the library supports a
simple counter for which no other statistics are generated.  This can be
useful for counting events, for example.

This library also implements a form of hierarchical statistics.  Such
structures can provide multiple levels of statistics to help reduce
loss of information for long-running programs.  See the hier.rs file and
documentation for more information.

Rustics also implements sets that contain statistics instances.  All the
statistics in a set can be printed via the print() method of the set.
Rustics provides one set type sets that contains Rustics objects as
Arc<Mutex<Rustics>> structures another set type that uses structures
of type Rc<RefCell<Rustics>>.  The two set modules are otherwise largely
identical.

The implementations allows the deletion of an element of a set, but the
deletion is done with a linear search and thus will not scale to handle a
single set with a very large number of elements.

Sets are recursive:  a set can have a set as a member.  A set hierarchy can be
printed with a single procedure call, and the contained statistics object can
be cleared in the same way.

The set implementation automatically creates hierarchical names for objects.
Each set or statistic typically will have a single element name given by the
user, and the string "==>" will be used to create a hierarchical name in the
manner of a Unix filesystem by concatenating the names as the print process
descends the tree.  For example, if a user has one set named "Everything" and
that set contains one statistics object named "Lonely", the title printed will
be "Everything ==> Lonely".
