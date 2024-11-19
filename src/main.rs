use rustics::Rustics;
use rustics::Histogram;
use rustics::PrintOpts;
use rustics::Units;
use rustics::printer_mut;
use rustics::stdout_printer;
use rustics::running_integer::RunningInteger;
use rustics::running_float::RunningFloat;

//
// This program is a very minimal example of how to use the
// Rustics library.  Note that the code prints the histogram
// twice, once via the Rustics print() method, and once via
// the print_histogram(), just by way of example.
//

fn main() {
    // Create an instance to record packet sizes. Set some print
    // options as an example.  Only float histograms have options,
    // so that field can be None.

    let printer    = Some(stdout_printer());
    let title      = Some("Network Packet Sizes".to_string());
    let units      = Some(Units::new("byte", "bytes"));
    let histo_opts = None;

    let print_opts = PrintOpts { printer, title, units, histo_opts };

    let mut packet_sizes = RunningInteger::new("Packet Sizes", &Some(print_opts));

    // Record some hypothetical packet sizes.

    let sample_count = 1000;

    for i in 1..=sample_count {
       packet_sizes.record_i64(i);
       assert!(packet_sizes.count() == i as u64);
    }

    // Print our statistics.

    packet_sizes.print();

    // Print just the histogram.  This example shows how the printer code
    // works.

    let printer = stdout_printer();      // create a shareable printer
    let printer = printer_mut!(printer); // get the printer out of the cell

    packet_sizes.print_histogram(printer);

    // We should have seen "sample_count" events.

    assert!(packet_sizes.count() == sample_count as u64);

    // Compute the expected mean.  We need the sum of
    //     1 + 2 + ... + n
    // which is
    //     n * (n + 1) / 2.

    let float_count = sample_count as f64;
    let float_sum   = float_count * (float_count + 1.0) / 2.0;
    let mean        = float_sum / float_count;

    assert!(packet_sizes.mean() == mean);

    // Let's record more samples, and verify the sample count as we go.

    let next_sample_count = 100;

    for i in 1..=next_sample_count {
       packet_sizes.record_i64(i + sample_count);
       assert!(packet_sizes.count() == (sample_count + i) as u64);
    }

    let mut samples = RunningFloat::new("f64 Samples", &None);

    for i in 1..=sample_count {
       samples.record_f64(i as f64);
       assert!(samples.count() == i as u64);
    }

    assert!(samples.count() == sample_count as u64);
    assert!(samples.mean()  == mean               );

    samples.print();
}
