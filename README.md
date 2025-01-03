# Rustics
A minimal statistics library for performance monitoring

Rustics implements statistics structs intended primarily for measuring
performance parameters.  The struct implementations compute various statistical
characterizations of the data, such as the mean and variance.  The trait
Rustics defines the common interface for all statistical types.

At this time, statistics can be gathered for i64 values, time periods (also of
type i64), and f64 samples.  Statistics can gathered collected as a running
total of all samples recorded, or as a window covering the last N samples.  The
library also supports creating sets of structs.

Most modules have sample code in their documentation, and many testing modules
contain example functions called "sample_usage".  The "main.rs" program
contains a small example of using running integer and time statistics structs
in a set.  You can run it by invoking "cargo run".

The structs for time samples require a timer implementation.  See the time
module for information on timers and to see some sample implementations.

In addition to the more detailed statistics types, the library supports a
simple counter for which no other statistics are generated.  This can be
useful for counting events, for example.

This library also implements a form of hierarchical statistics.  The
hierarchical statistics combine sets of Rustics instances into single
Rustics instances.  This summation can allow keeping historical data with
a lower memory footprint, and can reduce floating-point loss of precision.
See the lib.rs comments for an overview.

Rustics also implements sets that contain Rustics instances.  All the
instances in a set can be printed via the print() method of the set, and
other functions are provided, as well.  Rustics provides one set type that
contains Rustics instances as Arc\<Mutex\<dyn Rustics\>\> structures and 
another set type that uses structures of type Rc\<RefCell\<dyn Rustics\>\>.
The two set modules are otherwise largely identical.

Sets are recursive:  a set can have a set as a member.  All the Rustics
instances in a set hierarchy can be printed with a single method call, and
the contained Rustics instances can be cleared with a single method, as
well.

The set implementation automatically creates hierarchical titles for
Rustics instances.  For example, if a user has a set named "Network
Statistics" and that set contains a Rustics instance named "Packet Latency",
the title printed for that instance will be "Network Statistics ==\> Packet
Latency".
