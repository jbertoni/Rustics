//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

// These structures and routines are common code for printing
// statistics.
//
// The Printable struct allows passing common values to be
// printed to generic routines for RunningInteger and
// IntegerWindow structs.

use super::Printer;

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
    // Insert commas into a string containing an integer.

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

        let result =
            match sign {
                "+" => "+".to_string() + &result,
                "-" => "-".to_string() + &result,
                _   => result,
            };

        result
    }

    // Convert an i64 into a string with comma separators.

    pub fn commas_i64(value: i64) -> String {
        let base = value.to_string();

        Self::commas(&base)
    }

    // Convert an u64 into a string with comma separators.

    pub fn commas_u64(value: u64) -> String {
        let base = value.to_string();

        Self::commas(&base)
    }

    // Format a time value for printing.

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
    pub fn print_integer(name: &str, value: i64, printer: &mut dyn Printer) {
        let output = format!("    {:<12} {:>12}", name, Self::commas_i64(value));
        printer.print(&output);
    }

    pub fn print_float(name: &str, value: f64, printer: &mut dyn Printer) {
        Self::print_float_unit(name, value, "", printer)
    }

    pub fn print_float_unit(name: &str, value: f64, unit: &str, printer: &mut dyn Printer) {
        assert!(!value.is_nan());

        let value = format!("{:+e}", value)
            .replace('e', " e+")
            .replace("e+-", " e-") ;

        let mantissa_digits = 8;
        let mut mantissa = Vec::with_capacity(mantissa_digits);

        for char in value.chars() {
            if char == ' ' {
                break;
            }

            mantissa.push(char);

            if mantissa.len() == 8 {
                break;
            }
        }

        while mantissa.len() < mantissa_digits {
            mantissa.push('0');
        }

        let mantissa: String = mantissa.into_iter().collect();
        let exponent         = value.split(' ').last().unwrap();
        let output           = format!("    {:<13}    {} {} {}", name, mantissa, exponent, unit);

        printer.print(&output);
    }

    pub fn print_time(name: &str, time: f64, hz: i64, printer: &mut dyn Printer) {
        let (scaled_time, unit) = Self::scale_time(time, hz);

        if scaled_time > 999999.0 {
            Self::print_float_unit(name, scaled_time, &unit, printer);
        } else {
            let output = format!("    {:<12} {:>12.3} {}", name, scaled_time, unit);
            printer.print(&output);
        }
    }

// Compute the sample variance.

    // Print the common integer statistics as passed in a Printable structure.

    pub fn print_common_integer(&self, printer: &mut dyn Printer) {
        Self::print_integer("Count", self.n as i64, printer);

        if self.n > 0 {
            Self::print_integer("Minumum",  self.min,      printer);
            Self::print_integer("Maximum",  self.max,      printer);
            Self::print_integer("Log Mode", self.log_mode, printer);
        }
    }

    // Print the common computed statistics as passed in a Printable structure.
    // This includes values like the mean, which should be limited to an integer
    // value.

    pub fn print_common_float(&self, printer: &mut dyn Printer) {
        if self.n > 0 {
            Self::print_float("Mean",     self.mean,            printer);
            Self::print_float("Std Dev",  self.variance.sqrt(), printer);
            Self::print_float("Variance", self.variance,        printer);
            Self::print_float("Skewness", self.skewness,        printer);
            Self::print_float("Kurtosis", self.kurtosis,        printer);
        }
    }

    pub fn print_common_integer_times(&self, hz: i64, printer: &mut dyn Printer) {
        Self::print_integer("Count", self.n as i64, printer);

        if self.n > 0 {
            Self::print_time("Minumum",  self.min as f64,      hz, printer);
            Self::print_time("Maximum",  self.max as f64,      hz, printer);
            Self::print_time("Log Mode", self.log_mode as f64, hz, printer);
        }
    }

    pub fn print_common_float_times(&self, hz: i64, printer: &mut dyn Printer) {
        if self.n > 0 {
            Self::print_time("Mean",     self.mean,            hz, printer);
            Self::print_time("Std Dev",  self.variance.sqrt(), hz, printer);
            Self::print_time("Variance", self.variance,        hz, printer);
            Self::print_time("Skewness", self.skewness,        hz, printer);
            Self::print_time("Kurtosis", self.kurtosis,        hz, printer);
        }
    }
}
