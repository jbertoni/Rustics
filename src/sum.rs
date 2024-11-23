//
//  Copyright 2024 Jonathan L Bertoni
//
//  This code is available under the Berkeley 2-Clause, Berkely 3-clause,
//  and MIT licenses.
//

//  Sort a set by absolute value to try to improve the accuracy of summation.

fn sort(input: &mut [f64]) {
    input.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
}

/// The kbk_sum_sort() function performs a Kahan-Babushka-Klein summation.
/// It has a sort on the inputs in addition to the normal algorithm.
///
/// See the Wikipedia page on Kahan summation for the details of this
/// algorithm.

pub fn kbk_sum_sort(input: &mut [f64]) -> f64 {
    sort(input);
    kbk_sum(input)
}

/// The kbk_sum() function performs a Kahan-Babushka-Klein summation.  It does
/// not sort its inputs.  Currently, it is used by code that has presorted
/// vectors.

pub fn kbk_sum(input: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut cs  = 0.0;
    let mut ccs = 0.0;

    for addend in input {
        let     addend = *addend;
        let mut t      = sum + addend;

        let c =
            if sum.abs() >=  addend.abs() {
                (sum - t) +  addend
            } else {
                (addend - t) + sum
            };

        sum = t;
        t   = cs + c;

        let cc =
            if cs.abs() >= c.abs() {
                (cs - t) + c
            } else {
                (c - t) + cs
            };

        cs   = t;
        ccs += cc;
    }

    sum + cs + ccs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_tests() {
        let     limit  = 16;
        let mut inputs = Vec::<f64>::new();

        for i in 0..=limit {
            inputs.push(i as f64);
        }

        let result = kbk_sum(&inputs);

        //  Compute the expected value of the sum

        let expected = ((limit + 1) * limit) / 2;

        //  Now check what we got.

        assert!(result == expected as f64);

        // The version with the sort should match...

        let result = kbk_sum_sort(&mut inputs);
        assert!(result == expected as f64);

        println!("vector sum 1:  {}", result);

        //  Now add some negative values to the vector.

        for i in 0..=limit {
            inputs.push(-i as f64);
        }

        let result = kbk_sum(&inputs);

        println!("vector sum 2:  {}", result);

        assert!(result == 0.0);

        // Use the sort...

        let result = kbk_sum_sort(&mut inputs);

        assert!(result == 0.0);

        //  Run the test example from Wikipedia:
        //    [ 1, large, -1, large ]

        let large = (10.0 as f64).powi(100);

        inputs.clear();
        inputs.push(1.0);
        inputs.push(large);
        inputs.push(1.0);
        inputs.push(-large);

        let result = kbk_sum(&inputs);

        assert!(result == 2.0);

        let result = kbk_sum_sort(&mut inputs);

        println!("vector sum 3:  {}", result);
        assert!(result == 2.0);
    }
}
