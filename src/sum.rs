//
//  This code is available under the Berkeley 2-Clause, Berkely 2-clause,
//  and MIT licenses.  It is also available as public domain source where
//  permitted by law.
//

//  Sort a set by absolute value to improve the accuracy of summation.

pub fn sort(input: &mut [f64]) {
    input.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
}

// Implement a Kahan-Babushka-Klein summation routine.  Add a
// sort on the inputs.  For our purposes, the sort is low enough
// in cost.  See the Wikipedia page on Kahan summation for
// details.

pub fn sum(input:  &mut [f64]) -> f64 {
    sort(input);

    let mut sum = 0.0;
    let mut cs  = 0.0;
    let mut ccs = 0.0;

    for i in 0..input.len() {
        let mut t = sum + input[i];

        let c =
            if sum.abs() >= input[i].abs() {
                (sum - t) + input[i]
            } else {
                (input[i] - t) + sum
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
    pub fn run_tests() {
        let     limit  = 16;
        let mut inputs = Vec::<f64>::new();

        for i in 0..limit + 1 {
            inputs.push(i as f64);
        }

        let result = sum(&mut inputs);
        println!("vector sum 1:  {}", result);

        //  Compute the expected value of the sum

        let expected = ((limit + 1) * limit) / 2;

        //  Now check what we got.

        assert!(result == expected as f64);

        //  Now add some negative values to the vector.

        for i in 0..limit + 1 {
            inputs.push(-i as f64);
        }

        let result = sum(&mut inputs);
        println!("vector sum 2:  {}", result);

        assert!(result == 0.0);

        //  Run the test example from Wikipedia:
        //    [ 1, large, -1, large ]

        let large = (10.0 as f64).powi(100);

        inputs.clear();
        inputs.push(1.0);
        inputs.push(large);
        inputs.push(1.0);
        inputs.push(-large);

        let result = sum(&mut inputs);

        println!("vector sum 3:  {}", result);
        assert!(result == 2.0);
    }
}
