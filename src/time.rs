//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//!
//! ## Types
//!
//! * Timer
//!   * Timer is the trait for time operations needed by the
//!     statistics functions.
//!
//! * DurationTimer
//!   * DurationTimer provides a Timer interface to the standard
//!     rust Duration type, which measures wall-clock time.
//!
//! * SimpleClock
//!   * SimpleClock is an abstraction that can be used to implement
//!     platform-specific Timer instances.  Something like a simple
//!     cycle counter would be an example.
//!
//! * ClockTimer
//!   * Clock timer is an implementation of Timer for a SimpleClock.
//!
//!   * ClockTimer uses a Rc<RefCell<dyn SimpleClock>> provided
//!     to ClockTimer::new to read time.
//!
//!```
//!     use rustics::time::SimpleClock;
//!
//!     // This is a example implementation of the SimpleClock trait.  It
//!     // simple returns a series of time values incrementing by one in
//!     // size per invocation.
//!
//!     struct ExampleClock {
//!         current_time: u128,
//!         hz:           u128,
//!     }
//!
//!     impl ExampleClock {
//!         fn new(start_time: u128, hz: u128) -> ExampleClock {
//!             let current_time = start_time;
//!
//!             ExampleClock { current_time, hz }
//!         }
//!     }
//!
//!     impl SimpleClock for ExampleClock {
//!         fn get_time(&mut self) -> u128 {
//!             self.current_time += 1;
//!             self.current_time
//!         }
//!
//!         fn hz(&self) -> u128 {
//!             self.hz
//!         }
//!     }
//!
//!     let     start_time  = 1;
//!     let     hz          = 1_000_000_000;
//!     let mut clock       = ExampleClock::new(start_time, hz);
//!
//!     // Get a few time values.
//!
//!     for i in 1..100 {
//!         let time = clock.get_time();
//!
//!         assert!(time == start_time + i);
//!     }
//!
//!     assert!(clock.hz() == hz);
//!```

use std::time::Instant;
use std::rc::Rc;
use std::cell::RefCell;

use crate::timer_box;

///  A Timer is an abstraction of a clock to be used for performance
///  monitoring.  It is intended to allow for many implementations.
///  The underlying clock implementation determines the meaning of an
///  interval value.  For example, a DurationTimer uses the standard
///  Rust Duration type, which returns wall-clock time.  It operates
///  in nanoseconds.

pub trait Timer {
    ///  The start method starts a timing interval.  It may be called
    ///  multiple times on a single instance.  The last invocation of
    ///  the start method overrides any previous calls.

    fn start(&mut self);            // start or restart a timer

    ///  The finish() function is used at the end of a sample interval.  It
    ///  returns the interval time in ticks and also starts a new
    ///  interval, since the restart cost is nearly zero.  Thus, finish()
    ///  can be called multiple times after a start() invocation to return
    ///  the times for a sequence of events.  If a more precise timing is
    ///  required, the user must invoke start().

    fn finish(&mut self) -> i64;    // get the elapsed time and set a new start time

    ///  hz returns the hertz of the underlying clock.

    fn hz(&self) -> u128;           // get the clock hz
}

pub type DurationTimerBox = Rc<RefCell<DurationTimer>>;
pub type ClockTimerBox    = Rc<RefCell<ClockTimer>>;

///  DurationTimer uses the Rust standard time struct Duration to
///  measure time intervals.  This timer thus returns wall-clock time.
///  It currently works in units of nanoseconds.

#[derive(Clone)]
pub struct DurationTimer {
    start:      Instant,
    previous:   u128,
}

impl Timer for DurationTimer {
    fn start(&mut self) {
        self.start    = Instant::now();
        self.previous = 0;
    }

    // Get the current elapsed time and subtract
    // "previous" from it to get the time since the
    // last "finish" call.  Then save this current
    // time as the new "previous".

    fn finish(&mut self) -> i64 {
        let end_time  = self.start.elapsed().as_nanos();
        let result    = end_time - self.previous;
        self.previous = end_time;

        std::cmp::min(result, i64::MAX as u128) as i64
    }

    // We read the clock in nanoseconds currently.

    fn hz(&self) -> u128 {
        1_000_000_000
    }
}

impl DurationTimer {
    pub fn new() -> DurationTimer {
        let start    = Instant::now();
        let previous = 0;

        DurationTimer { start, previous }
    }

    pub fn new_box() -> DurationTimerBox {
        let timer = DurationTimer::new();

        timer_box!(timer)
    }
}

impl Default for DurationTimer {
    fn default() -> Self {
        Self::new()
    }
}

///  SimpleClock can be implemented for platform-specific clocks.
///  The instances can then be wrapped in a ClockTimer instance.

pub trait SimpleClock {
    /// Returns the current "time" in ticks, for whatever the
    /// time means for the clock being used.  A cycle counter
    /// will just return the current cycle count, for example.

    fn get_time(&mut self) -> u128;

    /// Returns the hertz of the clock.

    fn hz(&self) -> u128;
}

/// The ClockTimer type is a wrapper class for platform-specific
/// clocks.  It can be used for cycle counters and execution
/// time clocks, for example.

#[derive(Clone)]
pub struct ClockTimer {
    start:      u128,
    clock:      Rc<RefCell<dyn SimpleClock>>,
    hz:         u128,
}

impl Timer for ClockTimer {
    fn start(&mut self) {
        self.start = self.clock.borrow_mut().get_time();
    }

    fn finish(&mut self) -> i64 {
        let end_time = self.clock.borrow_mut().get_time();
        let ticks    = end_time - self.start;
        self.start   = end_time;

        ticks as i64
    }

    fn hz(&self) -> u128 {
        self.hz
    }
}

impl ClockTimer {
    pub fn new(clock: Rc<RefCell<dyn SimpleClock>>) -> ClockTimer {
        let start = clock.borrow_mut().get_time();
        let hz    = clock.borrow().hz();

        ClockTimer { start, clock, hz }
    }

    pub fn new_box(clock: Rc<RefCell<dyn SimpleClock>>) -> ClockTimerBox {
        let timer = ClockTimer::new(clock);

        timer_box!(timer)
    }
}

/// The timer_box macro converts a DurationTimerBox or a ClockTimerBox
/// instance into the shareable form, currently Rc<RefCell<_>>.

#[macro_export]
macro_rules! timer_box { ($x:expr) => { Rc::from(RefCell::new($x)) } }

/// The timer macro converts a DurationTimerBox or a ClockTimerBox into
/// a Timer instance.

#[macro_export]
macro_rules! timer { ($x:expr) => { &*$x.borrow() } }

/// The timer macro converts a DurationTimerBox or a ClockTimerBox into
/// a mutable Timer instance.

#[macro_export]
macro_rules! timer_mut { ($x:expr) => { &mut *$x.borrow_mut() } }

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    fn simple_duration_test() {
        let mut clock         = DurationTimer::new();
        let     seconds       = 1;
        let     sleep_time    = Duration::new(seconds, 0);
        let     base_interval = seconds as i64 * clock.hz() as i64;

        clock.start();

        for _i in 1..10 {
            sleep(sleep_time);
            let interval = clock.finish();

            assert!(interval >= base_interval);
            assert!(interval < base_interval + (base_interval / 20));
        }
    }

    struct TestSimpleClock {
        pub current:    u128,
        pub increment:  u128,
    }

    impl SimpleClock for TestSimpleClock {
        fn get_time(&mut self) -> u128 {
            let result = self.current;

            self.current   = self.current + self.increment;
            self.increment = self.increment * 2;
            result
        }

        fn hz(&self) -> u128 {
            1_000_000_000
        }
    }

    pub fn simple_test_clock() {
        let     current      = 0;
        let mut increment    = 1500;
        let     simple_clock = timer_box!(TestSimpleClock { current, increment });
        let     clock        = ClockTimer::new_box(simple_clock);
        let     clock        = timer_mut!(clock);

        // Creating the clock invokes get_time, so the increment in the
        // test clock increases.  Keep ours in sync with it.

        increment = increment * 2;

        assert!(clock.hz() == 1_000_000_000);

        clock.start();

        for _i in 1..5 {
            let interval = clock.finish();
            assert!(interval == increment as i64);

            // Keep our increment in sync with the test clock.
            increment = increment * 2;
        }
    }

    // This is a example implementation of the SimpleClock
    // trait.  It simple returns a series of time values
    // incrementing by one in size per invocation.

    struct ExampleClock {
        current_time: u128,
        hz:           u128,
    }

    impl ExampleClock {
        fn new(start_time: u128, hz: u128) -> ExampleClock {
            let current_time = start_time;

            ExampleClock { current_time, hz }
        }
    }

    impl SimpleClock for ExampleClock {
        fn get_time(&mut self) -> u128 {
            self.current_time += 1;
            self.current_time
        }

        fn hz(&self) -> u128 {
            self.hz
        }
    }

    fn example_clock() {
        let     start_time  = 1;
        let     hz          = 1_000_000_000;
        let mut clock       = ExampleClock::new(start_time, hz);

        // Get a few time values.

        for i in 1..100 {
            let time = clock.get_time();

            assert!(time == start_time + i);
        }

        assert!(clock.hz() == hz);
    }

    fn simple_default_test() {
        let mut timer = DurationTimer::default();

        for _i in 1..100 {
            let nanoseconds = timer.finish();
            assert!(nanoseconds >= 0)
        }
    }

    #[test]
    pub fn run_tests() {
        simple_duration_test();
        simple_default_test();
        simple_test_clock();
        example_clock();
    }
}
