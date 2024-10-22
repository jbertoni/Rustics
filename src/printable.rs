//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//!
//! ## Type
//!
//! * Printable
//!     * Printable implement the output formatting used by all the
//!       statistics types, like RunningInteger.
//      * This module provides helper functions for formatting integers
//!       and time values.
//!
//!```
//!     use rustics::printable::Printable;
//!
//!
//!     let hz       = 1_000_000_000;
//!     let second   = hz as f64;
//!     let ms       = second / 1000.0;
//!     let us       = second / 1000_000.0;
//!     let ns       = second / 1_000_000_000.0;
//!     let minute   = second * 60.0;
//!     let hour     = minute * 60.0;
//!     let day      = 24.0 * hour;
//!     let week     = day * 7.0;
//!
//!     let examples =
//!         [
//!             (         100.0,   100.0,  "ns"     ),
//!             (         102.4,   102.4,  "ns"     ),
//!             (       1_000.0,     1.0,  "us"     ),
//!             (       1_200.0,     1.2,  "us"     ),
//!             (      29_000.0,    29.0,  "us"     ),
//!             (   1_000_000.0,     1.0,  "ms"     ),
//!             (  29_000_000.0,    29.0,  "ms"     ),
//!
//!             (       us - ns,   999.0,  "ns"     ),
//!             (       ms - us,   999.0,  "us"     ),
//!             (   second - ms,   999.0,  "ms"     ),
//!
//!             (        second,     1.0,  "second" ),
//!             (  1.5 * second,     1.5,  "seconds"),
//!             (  3.0 * second,     3.0,  "seconds"),
//!             (  3.0 * second,     3.0,  "seconds"),
//!             ( 42.0 * second,    42.0,  "seconds"),
//!             (          hour,     1.0,  "hour"   ),
//!             (   12.6 * hour,    12.6,  "hours"  ),
//!             (           day,     1.0,  "day"    ),
//!             (     2.0 * day,     2.0,  "days"   ),
//!             (   999.0 * day,   999.0,  "days"   ),
//!             (          week,     7.0,  "days"   ),
//!         ];
//!
//!     // Convert a time in ticks to a scaled value and
//!     // and a string for units.  For example, 1000 ns
//!     // should return (1.0, "us").
//!
//!     for example in examples {
//!         let (ticks, time, units) = example;
//!
//!         let (result_time, result_units) =
//!             Printable::scale_time(ticks, hz);
//!
//!         println!(
//!             "documentation:  expect ({} {}) from {}, got ({} {})",
//!             time, units,
//!             ticks,
//!             result_time, result_units
//!         );
//!
//!         assert!(result_time  == time);
//!         assert!(result_units == units);
//!
//!         // The commas functions works to add commas to integer output.
//!         // It handles "+" and "-" signs.  The interface functions
//!         // commas_64 and commas_u64 are a bit more convenient to use.
//!
//!         assert_eq!(Printable::commas(   "+20"),       "+20");
//!         assert_eq!(Printable::commas(  "-200"),      "-200");
//!         assert_eq!(Printable::commas(  "2000"),     "2,000");
//!         assert_eq!(Printable::commas("+12345"),   "+12,345");
//!         assert_eq!(Printable::commas("-12345"),   "-12,345");
//!         assert_eq!(Printable::commas("200000"),   "200,000");
//!     }
//!```

// The Printable struct and associated functions are common code for
// printing statistics.
//
// The Printable struct allows passing common values to be printed to
// generic functions for RunningInteger and IntegerWindow instances.

use super::Printer;

/// The Printable struct is used to pass data to the standard print
/// functions shared by all the code.  Developers who are implementing
/// the Rustics trait for a new type might use this module.

#[derive(Copy, Clone)]
pub struct Printable {
    pub n:          u64,
    pub min:        i64,
    pub max:        i64,
    pub log_mode:   i64,
    pub mean:       f64,
    pub variance:   f64,
    pub skewness:   f64,
    pub kurtosis:   f64,
}

impl Printable {
    /// The commas() function inserts  commas into a string
    /// containing the character form of an integer.  This
    /// input string might or might not have a leading "+" or
    /// "-" sign.

    pub fn commas(value: &str) -> String {
        if value.len() <= 3 {
            return value.to_string()
        }

        let sign;
        let digits;
        let comma_interval = 3;

        //  A string like "-200" shouldn't be printed as "-,200", so detect and
        //  handle leading signs that'll cause a comma to be added.  If the
        // string length is 1 mod 3 and the top character is a sign, we need to
        // intervene.

        if value.len() % comma_interval == 1 {
            match value.chars().next().unwrap() {
                '+' => { sign = "+"; digits = value[1..].to_string(); }
                '-' => { sign = "-"; digits = value[1..].to_string(); }
                _   => { sign = ""; digits = value.to_string(); }
            }
        } else {
            sign   = "";
            digits = value.to_string()
        }

        let result =
            digits
                .as_bytes()                 // convert the input to a byte array
                .rchunks(comma_interval)    // break into chunks of three (or whatever) from the right
                .rev()                      // reverse the current order back to the original order
                .map(std::str::from_utf8)   // convert back to a vector of strings
                .collect::<Result<Vec<&str>, _>>()
                .unwrap()
                .join(",");                 // join the blocks of three digits with commas

        // Add the sign back in front as needed.

        match sign {
            "+" => "+".to_string() + &result,
            "-" => "-".to_string() + &result,
            _   => result,
        }
    }

    /// Converts an i64 into a string with comma separators.

    pub fn commas_i64(value: i64) -> String {
        let base = value.to_string();

        Self::commas(&base)
    }

    /// Converts a u64 into a string with comma separators.

    pub fn commas_u64(value: u64) -> String {
        let base = value.to_string();

        Self::commas(&base)
    }

    /// scale_time() converts a time value in clock ticks into a
    /// human-readable value and unit.  The chosen unit is returned
    /// as a string for printing.

    pub fn scale_time(time: f64, hz: i64) -> (f64, String) {
        let microsecond = 1_000.0;
        let millisecond = microsecond * 1000.0;
        let second      = millisecond * 1000.0;
        let minute      = 60.0 * second;
        let hour        = 60.0 * minute;
        let day         = hour * 24.0;

        // Convert the time to nanoseconds

        let time = time * (1_000_000_000.0 / hz as f64);

        let unit;
        let scale;
        let has_plural;

        // Decide what units to use.

        if time >= day {
            unit       = "day";
            scale      = day;
            has_plural = true;
        } else if time >= hour {
            unit       = "hour";
            scale      = hour;
            has_plural = true;
        } else if time >= minute {
            unit       = "minute";
            scale      = minute;
            has_plural = true;
        } else if time >= second {
            unit       = "second";
            scale      = second;
            has_plural = true;
        } else if time >= millisecond {
            unit       = "ms";
            scale      = millisecond;
            has_plural = false;
        } else if time >= microsecond {
            unit       = "us";
            scale      = microsecond;
            has_plural = false;
        } else {
            unit       = "ns";
            scale      = 1.0;
            has_plural = false;
        }

        let plural = time != scale;

        let suffix =
            if plural & has_plural {
                "s"
            } else {
                ""
            };

        let     scaled_time = time / scale;
        let mut unit        = unit.to_string();

        unit.push_str(suffix);

        (scaled_time, unit)
    }

    /// Prints an integer statistic and its name in the standard format.

    pub fn print_integer(name: &str, value: i64, printer: &mut dyn Printer) {
        let output = format!("    {:<12} {:>12}", name, Self::commas_i64(value));
        printer.print(&output);
    }

    /// Prints an integer statistic and its name in the standard format.

    pub fn print_float(name: &str, value: f64, printer: &mut dyn Printer) {
        Self::print_float_unit(name, value, "", printer)
    }

    /// Prints a float value and its name along with a string specifying
    /// the unit.

    pub fn print_float_unit(name: &str, value: f64, unit: &str, printer: &mut dyn Printer) {
        assert!(!value.is_nan());

        // Print the value in scientific notation, then
        // force a sign onto the exponent to make things
        // line up.

        let value = format!("{:+e}", value)
            .replace('e',   " e+")
            .replace("e+-", " e-") ;

        // Force the mantissa to 8 digits.  This should
        // help with legibility since all the numbers
        // should align.

        let     mantissa_digits = 8;
        let mut mantissa        = Vec::with_capacity(mantissa_digits);

        for char in value.chars() {
            if char == ' ' {
                break;
            }

            mantissa.push(char);

            if mantissa.len() == 8 {
                break;
            }
        }

        // Add trailing zeroes as needed.

        while mantissa.len() < mantissa_digits {
            mantissa.push('0');
        }

        let mantissa: String = mantissa.into_iter().collect();
        let exponent         = value.split(' ').last().unwrap();
        let output           = format!("    {:<13}    {} {} {}", name, mantissa, exponent, unit);

        printer.print(&output);
    }

    /// Prints a time value in human-usable form.

    pub fn print_time(name: &str, time: f64, hz: i64, printer: &mut dyn Printer) {
        let (scaled_time, unit) = Self::scale_time(time, hz);

        if scaled_time > 999999.0 {
            Self::print_float_unit(name, scaled_time, &unit, printer);
        } else {
            let output = format!("    {:<12} {:>12.3} {}", name, scaled_time, unit);
            printer.print(&output);
        }
    }

    /// Prints the common integer statistics as passed in a Printable instance.

    pub fn print_common_integer(&self, printer: &mut dyn Printer) {
        Self::print_integer("Count", self.n as i64, printer);

        if self.n > 0 {
            Self::print_integer("Minumum",  self.min,      printer);
            Self::print_integer("Maximum",  self.max,      printer);
            Self::print_integer("Log Mode", self.log_mode, printer);
        }
    }

    /// Prints the common float statistics as passed in a Printable instance.
    /// This includes values like the mean, which should be limited to an integer
    /// value.

    pub fn print_common_float(&self, printer: &mut dyn Printer) {
        if self.n > 0 {
            Self::print_float("Mean",     self.mean,            printer);
            Self::print_float("Std Dev",  self.variance.sqrt(), printer);
            Self::print_float("Variance", self.variance,        printer);
            Self::print_float("Skewness", self.skewness,        printer);
            Self::print_float("Kurtosis", self.kurtosis,        printer);
        }
    }

    /// log_mode_to_time converts the log_mode of a time-based histogram
    /// into an approximate time for the bucket.  Note that this
    /// approximation can be bigger than the record max value since
    /// the pseudo-log function rounds up.

    pub fn log_mode_to_time(&self) -> f64 {
        // Time values should never be negative...

        if self.log_mode < 0 {
            return 0.0;
        }

        let log   = self.log_mode.abs();
        let base  = 2_u64;
        let ticks = base.pow(log as u32);

        // Compute the approximate time interval for this number of ticks.

        ticks as f64
    }

    /// Prints integer values that are in time units as actual times. The
    /// mode of the pseudo-log is an exception.

    pub fn print_common_integer_times(&self, hz: i64, printer: &mut dyn Printer) {
        Self::print_integer("Count", self.n as i64, printer);

        if self.n > 0 {
            let approximation = self.log_mode_to_time();

            Self::print_time("Minumum",  self.min as f64, hz, printer);
            Self::print_time("Maximum",  self.max as f64, hz, printer);
            Self::print_time("Log Mode", approximation,   hz, printer);
        }
    }

    /// Prints the common f64 summary statistics.

    pub fn print_common_float_times(&self, hz: i64, printer: &mut dyn Printer) {
        if self.n > 0 {
            Self::print_time ("Mean",     self.mean,            hz, printer);
            Self::print_time ("Std Dev",  self.variance.sqrt(), hz, printer);
            Self::print_float("Variance", self.variance,            printer);
            Self::print_float("Skewness", self.skewness,            printer);
            Self::print_float("Kurtosis", self.kurtosis,            printer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_commas() {
        let test   = [ 123456, 12, -1, -1234, 4000000, -200, -2000, -20000 ];
        let expect =
            [ "123,456", "12", "-1", "-1,234", "4,000,000", "-200", "-2,000", "-20,000" ];

        for i in 0..test.len() {
            println!("Test:  {} vs {}", Printable::commas_i64(test[i]), expect[i]);

            assert_eq!(Printable::commas_i64(test[i]), expect[i]);
        }

        assert_eq!(Printable::commas(    "+21"),      "+21");
        assert_eq!(Printable::commas(   "+212"),     "+212");
        assert_eq!(Printable::commas(  "+2123"),   "+2,123");
        assert_eq!(Printable::commas( "+21234"),  "+21,234");
        assert_eq!(Printable::commas("+212345"), "+212,345");

        assert_eq!(Printable::commas(    "+20"),      "+20");
        assert_eq!(Printable::commas(   "+200"),     "+200");
        assert_eq!(Printable::commas(  "+2000"),   "+2,000");
        assert_eq!(Printable::commas( "+20000"),  "+20,000");
        assert_eq!(Printable::commas("+200000"), "+200,000");
    }

    fn test_log_mode_to_time() {
        let n        =  100;
        let min      =    1;
        let max      = 1000;
        let log_mode =   32;

        let mean     = 10.0;
        let variance = 10.0;
        let skewness = -4.0;
        let kurtosis = 10.0;

        let base     = 2 as u64;
        let expected = base.pow(log_mode as u32) as f64;

        let mut printable =
            Printable { n, min, max, log_mode, mean, variance, skewness, kurtosis };

        assert!(printable.log_mode_to_time() == expected);

        // Negative times aren't actually possible, but check the sanity test.

        printable.log_mode = -printable.log_mode;

        assert!(printable.log_mode_to_time() == 0.0);

        printable.log_mode = 63;

        let base     = 2 as u64;
        let expected = base.pow(63) as f64;

        assert!(printable.log_mode_to_time() == expected);
    }

    fn documentation() {
        let hz       = 1_000_000_000;
        let second   = hz as f64;
        let ms       = second / 1000.0;
        let us       = second / 1000_000.0;
        let ns       = second / 1_000_000_000.0;
        let minute   = second * 60.0;
        let hour     = minute * 60.0;
        let day      = 24.0 * hour;
        let week     = day * 7.0;

        let examples =
            [
                (          100.0,   100.0,  "ns"     ),
                (          102.4,   102.4,  "ns"     ),
                (        1_000.0,     1.0,  "us"     ),
                (        1_200.0,     1.2,  "us"     ),
                (       29_000.0,    29.0,  "us"     ),
                (    1_000_000.0,     1.0,  "ms"     ),
                (   29_000_000.0,    29.0,  "ms"     ),

                (        us - ns,   999.0,  "ns"     ),
                (        ms - us,   999.0,  "us"     ),
                (    second - ms,   999.0,  "ms"     ),
                (minute - second,    59.0,  "seconds"),
                (  hour - minute,    59.0,  "minutes"),
                (     day - hour,    23.0,  "hours"  ),

                (   3.0 * second,     3.0,  "seconds"),
                (   3.0 * second,     3.0,  "seconds"),
                (   1.5 * second,     1.5,  "seconds"),
                (  42.0 * second,    42.0,  "seconds"),
                (    999.0 * day,   999.0,  "days"   ),
                (    12.6 * hour,    12.6,  "hours"  ),
                (           week,     7.0,  "days"   ),

                (         second,     1.0,  "second" ),
                (   2.0 * second,     2.0,  "seconds"),
                (         minute,     1.0,  "minute" ),
                (   2.0 * minute,     2.0,  "minutes"),
                (           hour,     1.0,  "hour"   ),
                (     2.0 * hour,     2.0,  "hours"  ),
                (            day,     1.0,  "day"    ),
                (      2.0 * day,     2.0,  "days"   ),
            ];

        for example in examples {
            let (ticks, time, unit) = example;

            let (result_time, result_unit) = Printable::scale_time(ticks, hz);

            println!("documentation:  expect ({} {}) from {}, got ({} {})",
                time, unit,
                ticks,
                result_time, result_unit
            );

            assert!(result_time  == time);
            assert!(result_unit  == unit);
        }
    }

    #[test]
    fn run_tests() {
        test_commas();
        test_log_mode_to_time();
        documentation();
    }
}
