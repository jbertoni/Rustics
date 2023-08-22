# Rustics
A minimal statistics library for performance monitoring

Rustics implements a simple set of statistics objects intended primarly
for measuring performance parameters.  Data samples can be recorded in
these objects, and the library then computes various statistical
characterizations of the data, such as the mean and variance.

This library also implements sets, which contain statistics that are
printed and manipulated together.  Sets are recursive:  a set can have
a set as a member.  A set hierarchy can be printed with a single
procedure call, and the contained statistics object can be cleared in
the same way.  The set implementation automatically creates hierarchical
names for objects.  Each set or statistic typically will have a single
element name, and the period (".") will be used to create a hierchical
name in the manner of a Unix filesystem.

Statistics and sets can be deleted from their parent set, but the code
uses simple algorithms that are linear in the number of elements of a set
to do searching for items to be deleted, so sets with a very high number
of subsets or statistics can require a long time to process a deletion.
