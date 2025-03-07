//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkeley 3-clause,
//  and MIT licenses.
//

use rustics::Units;
use rustics::rc_sets::RcSet;
use rustics::time::DurationTimer;
use rustics::timer_mut;

// This program is a very minimal example of how to use the
// Rustics library.

fn main() {
    // Create a set, and create two Rustics instances in the set.

    let mut set          = RcSet::new("Network Statistics", 2, 0, &None);

    // The first statistic records statistics on packet sizes.

    let     units        = Some(Units::new("byte", "bytes"));
    let     packet_sizes = set.add_running_integer("Packet Size", units);

    // The second reports statistics on latency.

    let     timer        = DurationTimer::new_box();
    let     latencies    = set.add_running_time("Packet Latency", timer.clone());

    // Record some hypothetical packet sizes.

    let sample_count = 1000;

    for i in 1..=sample_count {
       // receive_packet();
       let packet_size = i; // dummy data for testing

       packet_sizes.borrow_mut().record_i64(packet_size);
    }

    // Record some hypothetical latencies.  Note that
    // record_event restarts the timer.

    timer_mut!(timer).start();

    for _i in 1..=sample_count {
        // receive_packet();

        latencies.borrow_mut().record_event();
    }

    // Print our statistics.

    println!(" === First print\n");
    set.print();

    // We should have seen "sample_count" events.

    assert!(packet_sizes.borrow().count() == sample_count as u64);
    assert!(latencies   .borrow().count() == sample_count as u64);

    // Compute the expected mean packet size.  We need the sum of
    //     1 + 2 + ... + n
    // which is
    //     n * (n + 1) / 2.

    let count = sample_count as f64;
    let sum   = count * (count + 1.0) / 2.0;
    let mean  = sum / count;

    assert!(packet_sizes.borrow().mean() == mean);

    // Demo the record_interval() interface.  Note that
    // record_interval queries the timer, which also restarts
    // the timer.

    let mut timer = DurationTimer::new_box();

    for _i in 1..=sample_count {
        latencies.borrow_mut().record_interval(&mut timer);
    }

    assert!(latencies.borrow().count() == 2 * sample_count as u64);
    println!("\n\n\n\n === Second print\n");
    set.print();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_main() {
        main();
    }
}
