use std::str::FromStr;

pub fn truncate_using_maths(float: f64, precision: u16) -> f64 {
    let float = float * (10.0f64.powi(precision as i32));

    // Truncate the mantissa
    let float = float.trunc();

    // Return the rate
    float / (10.0f64.powi(precision as i32))
}

pub fn truncate_using_string(float: f64, precision: u16) -> f64 {
    let mut string = float.to_string();

    let index = string.find('.');

    match index {
        None => float,
        Some(index) => {
            let trunc = index + 1 + precision as usize;
            string.truncate(trunc);
            f64::from_str(&string).expect("This should still be a number")
        }
    }
}

#[cfg(test)]
mod math_tests {
    use super::*;

    #[test]
    fn truncate() {
        let float = 1.123456789;

        assert_eq!(&truncate_using_maths(float, 5).to_string(), "1.12345");
    }
}

#[cfg(test)]
mod string_tests {
    use super::*;

    #[test]
    fn truncate() {
        let float = 1.123456789;

        assert_eq!(&truncate_using_string(float, 5).to_string(), "1.12345");
    }
}
