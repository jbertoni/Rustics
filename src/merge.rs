//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

/// Export is used by various modules to create sums of
/// statistics instances of type RunningInteger.

use std::rc::Rc;
use std::cell::RefCell;

use super::RecoverData;
use super::StatisticsData;
use super::recover;
use super::compute_statistics;
use super::sum::kbk_sum_sort;

use super::LogHistogramBox;
use super::FloatHistogramBox;

use super::log_histogram::LogHistogram;
use super::float_histogram::FloatHistogram;

#[derive(Clone)]
pub struct Export {
    pub count:      u64,
    pub nans:       u64,
    pub infinities: u64,
    pub mean:       f64,
    pub moment_2:   f64,
    pub cubes:      f64,
    pub moment_4:   f64,

    pub min_i64:    i64,
    pub max_i64:    i64,

    pub min_f64:    f64,
    pub max_f64:    f64,

    pub log_histogram:    Option<LogHistogramBox>,
    pub float_histogram:  Option<FloatHistogramBox>,
}

/// The sum_running() function merges a vector of exported statistics.

pub fn sum_running(exports: &Vec::<Export>) -> Export {
    let mut count           = 0;
    let mut nans            = 0;
    let mut infinities      = 0;
    let mut min_i64         = i64::MAX;
    let mut max_i64         = i64::MIN;
    let mut min_f64         = f64::MAX;
    let mut max_f64         = f64::MIN;

    let mut log_histogram   = LogHistogram::new();
    let mut is_log          = false;

    let mut float_histogram =
        if let Some(float_histogram) = &exports[0].float_histogram {
            let addend     = float_histogram.borrow();
            let print_opts = &addend.print_opts;
            
            FloatHistogram::new(print_opts)
         } else {
            is_log = true;

            FloatHistogram::new(&None)
         };

    let mut sum_vec          = Vec::with_capacity(exports.len());
    let mut squares_vec      = Vec::with_capacity(exports.len());
    let mut cubes_vec        = Vec::with_capacity(exports.len());
    let mut quads_vec        = Vec::with_capacity(exports.len());

    // Iterate through each set of exported data, gather merged
    // values.  We recover the squares and fourth powers of
    // each sample from data in the exports.

    for export in exports {
        count      += export.count;
        nans       += export.nans;
        infinities += export.infinities;
        min_i64     = std::cmp::min(min_i64, export.min_i64);
        max_i64     = std::cmp::max(max_i64, export.max_i64);

        if export.min_f64 < min_f64 {
            min_f64 = export.min_f64;
        }

        if export.max_f64 > max_f64 {
            max_f64 = export.max_f64;
        }

        if let Some(addend) = &export.log_histogram {
            let addend = addend.borrow();

            sum_log_histogram(&mut log_histogram, &addend);
        }

        if let Some(addend) = &export.float_histogram {
            let addend = addend.borrow();

            sum_float_histogram(&mut float_histogram, &addend);
        }

        let n        = export.count as f64;
        let mean     = export.mean;
        let moment_2 = export.moment_2;
        let cubes    = export.cubes;
        let moment_4 = export.moment_4;
        let data     = RecoverData { n, mean, moment_2, cubes, moment_4 };

        let (squares, quads) = recover(data);

        let sum = export.mean * n;

        sum_vec.push    (sum     );
        squares_vec.push(squares );
        cubes_vec.push  (cubes   );
        quads_vec.push  (quads   );
    }

    // Now merge the data that we got.  We get the sums
    // of the squares, cubes, and fourth power of each
    // original sample.  From that data, we compute
    // the merged 2nd and 4th moments about the mean,
    // as well as the mean.

    let n        = count as f64;
    let sum      = kbk_sum_sort(&mut sum_vec    [..]);
    let squares  = kbk_sum_sort(&mut squares_vec[..]);
    let cubes    = kbk_sum_sort(&mut cubes_vec  [..]);
    let quads    = kbk_sum_sort(&mut quads_vec  [..]);
    let data     = StatisticsData { n, sum, squares, cubes, quads };
    let merged   = compute_statistics(data);
    let mean     = merged.mean;
    let moment_2 = merged.moment_2;
    let moment_4 = merged.moment_4;

    // Okay, build the structure from which an instance
    // can be built.  First, box the log histogram.

    let log_histogram =
        if is_log {
            Some(Rc::from(RefCell::new(log_histogram)))
         } else {
            None
         };

    let float_histogram =
        if !is_log {
            Some(Rc::from(RefCell::new(float_histogram)))
        } else {
            None
        };

    Export {
        count,       mean,           moment_2,        cubes,    moment_4,
        min_i64,     max_i64,        min_f64,         max_f64,  nans,
        infinities,  log_histogram,  float_histogram
    }
}

/// sum_log_histogram() is used internally to create sums of
/// RunningInteger instances.

pub fn sum_log_histogram(sum:  &mut LogHistogram, addend: &LogHistogram) {
    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }
}

/// sum_float_histogram() is used internally to create sums of
/// RunningFloat instances.

pub fn sum_float_histogram(sum:  &mut FloatHistogram, addend: &FloatHistogram) {
    assert!(sum.negative.len() == addend.negative.len());
    assert!(sum.positive.len() == addend.positive.len());

    for i in 0..sum.negative.len() {
        sum.negative[i] += addend.negative[i];
    }

    for i in 0..sum.positive.len() {
        sum.positive[i] += addend.positive[i];
    }

    sum.nans       += addend.nans;
    sum.infinities += addend.infinities;
    sum.samples    += addend.samples;
}