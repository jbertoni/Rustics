# Rustics
A minimal statistics library for performance monitoring


Rustics implements a simple set of statistics types intended primarily for
measuring performance parameters.  The struct implementations compute
various statistical characterizations of the data, such as the mean and
variance.

Many testing modules contain very simple example functions called
"sample_usage", and most modules have sample code in their documentation,
as well.

At this time, statistics can be gathered for i64 values and for time
values.  Samples can be collected as a running total of all samples
recorded, or as a window covering the last N events, for a programmable
N.  The interface supports a "clear" function to discard all sample
that has been collected.

See the Rustics::time module for information on timers and to see some
sample implementations.

In addition to the more detailed statistics type, the library supports a
simple counter for which no other statistics are generated.  This can be
useful for counting events, for example.

This library also implements a form of hierarchical statistics.  The
hierarchical statistics combine fixed-size sets of samples into a
single statistics instance in a hierarchical fashion, to try to reduce
loss of information for long-running programs.  See the hier.rs file and
documentation for more information.

Rustics also implements sets that contain statistics instances.  All the
instances in a set can be printed via the print() method of the set.
Rustics provides one set type sets that contains Rustics instances as
Arc<Mutex<Rustics>> structures another set type that uses structures
of type Rc<RefCell<Rustics>>.  The two set modules are otherwise largely
identical.

The implementations allows the deletion of an element of a set, but the
deletion is done with a linear search and thus will not scale to handle a
single set with a very large number of elements.

Sets are recursive:  a set can have a set as a member.  A set hierarchy
can be printed with a single method call, and the contained statistics
instances can be cleared in the same way.

The set implementation automatically creates hierarchical titles for
statistics instances.  For example, if a user has a set named "Everything"
and that set contains one statistics instance named "Lonely", the title
printed for that instance will be "Everything ==> Lonely".
