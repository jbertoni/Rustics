//  At some point, we might want to sort by absolute value.

pub fn sort(input: &mut [f64]) {
    input.sort_by(|a, b| a.partial_cmp(b).unwrap())    
}

pub fn sum(input:  &mut [f64]) -> f64 {
    sort(input);

    let mut sum = 0.0;
    let mut cs  = 0.0;
    let mut ccs = 0.0;
    let mut c;
    let mut cc;

    for i in 0..input.len() {
        let mut t = sum + input[i];

        c = 
            if sum.abs() >= input[i].abs() {
                (sum - t) + input[i]
            } else {
                (input[i] - t) + sum
            };

        sum = t;
        t   = cs + c;

        cc =
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
        let mut inputs = Vec::<f64>::new();

        inputs.push(1.0);
        inputs.push(2.0);
        inputs.push(3.0);

        let result = sum(&mut inputs);
        println!("vector sum 1:  {}", result);
        assert!(result == 6.0);

        let large = (10.0 as f64).powi(100);

        inputs.clear();
        inputs.push(1.0);
        inputs.push(large);
        inputs.push(1.0);
        inputs.push(-large);

        let result = sum(&mut inputs);

        println!("vector sum 2:  {}", result);
        assert!(result == 2.0);
    }
}
