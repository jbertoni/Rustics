//  At some point, we might want to sort by absolute value.

pub fn sort(input: &mut [f64]) {
    input.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())    
}

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
    pub fn simple_test() {
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