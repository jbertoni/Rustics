use std::time::Instant;

//  A Timer is an abstraction of a clock to be used for performance
//  monitoring.  It is intended to allow for many implementations.
//  The underlying clock implementation determines the meaning of an
//  interval value.  For example, a DurationTimer uses the standard
//  Rust Duration type, which returns wall-clock time.
//
//  The start method starts a timing interval.  It may be called
//  multiple times on a single structure.  The last invocation of
//  the start method overrides any previous calls.
//
//  The finish routine is used at the end of a sample interval.  It
//  returns the interval time in nanoseconds and also starts a new
//  interval, since the restart cost is nearly zero.  Thus, "finish"
//  can be called multiple times after a "start" invocation to return
//  the times for a sequence of events.  If a more precise timing is
//  required, "start" will start an interval.
//
//  hz returns the herz of the underlying clock.

pub trait Timer {
    fn start(&mut self);            // start or restart a timer
    fn finish(&mut self) -> u128;   // get the elapsed time and set a new start time
    fn hz(&self) -> u128;           // get the clock hz
}


//  DurationTimer uses the Rust standard time function "Duration" to
//  measure time intervals.  This timer thus returns wall-clock time.

pub struct DurationTimer {
    start:      Instant,
    previous:   u128,
}

impl Timer for DurationTimer {
    fn start(&mut self) {
        self.start = Instant::now();
        self.previous = 0;
    }

    fn finish(&mut self) -> u128 {
        let end_time = self.start.elapsed().as_nanos();
        let result = end_time - self.previous;
        self.previous = end_time;
        result
    }

    fn hz(&self) -> u128 {
        1_000_000_000
    }
}

impl DurationTimer {
    pub fn new() -> DurationTimer {
        let start = Instant::now();
        let previous = 0;

        DurationTimer { start, previous }
    }
}

impl Default for DurationTimer {
    fn default() -> Self {
        Self::new()
    }
}

//  This trait can be implemented for platform-specific clocks.
//  The structures can then be wrapped in a ClockTimer struct.
//  See the test routine "simple_test_clock" for an example.

pub trait SimpleClock {
    fn get_time(&mut self) -> u128;
    fn hz(&self) -> u128;
}

//  This is a wrapper class for platform-specific clocks that
//  would be useful to support.
//
//  For efficiency, using 64-bit math internally might be useful.
//  On the other hand, using femtoseconds might be useful for
//  particularly hostile hz ratings.

pub struct ClockTimer {
    start:      u128,
    clock:      Box<dyn SimpleClock>,
}

impl Timer for ClockTimer {
    fn start(&mut self) {
        self.start = self.clock.get_time();
    }

    fn finish(&mut self) -> u128 {
        let end_time = self.clock.get_time();
        let ticks = end_time - self.start;
        self.start = end_time;
        ticks
    }

    fn hz(&self) -> u128 {
        self.clock.hz()
    }
}

impl ClockTimer {
    pub fn new(mut clock: Box<dyn SimpleClock>) -> ClockTimer {
        let start = clock.get_time();

        ClockTimer { start, clock }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    pub fn simple_test_duration() {
        let mut clock = DurationTimer::new();
        clock.start();
        let seconds = 1;
        let sleep_time = Duration::new(seconds, 0);
        let base_interval = seconds as u128 * clock.hz() as u128;

        for i in 1..10 {
            sleep(sleep_time);
            let interval = clock.finish();
            println!(" interval {} = {}", i, interval);
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
            self.current = self.current + self.increment;
            self.increment = self.increment * 2;
            result
        }

        fn hz(&self) -> u128 {
            1_000_000_000
        }
    }

    pub fn simple_test_clock() {
        let current = 0;
        let mut increment = 1500;
        let simple_clock = Box::new(TestSimpleClock { current, increment });
        let mut clock = ClockTimer::new(simple_clock);

        // Creating the clock invokes get_time, so the increment in the
        // test clock increases.  Keep ours in sync with it.

        increment = increment * 2;

        assert!(clock.hz() == 1_000_000_000);

        clock.start();

        for _i in 1..5 {
            let interval = clock.finish();
            println!("  result {} == predict {}", interval, increment);
            assert!(interval == increment);

            // Keep our increment in sync with the test clock.
            increment = increment * 2;
        }
    }

    #[test]
    pub fn simple_tests() {
        simple_test_duration();
        simple_test_clock();
    }
}
