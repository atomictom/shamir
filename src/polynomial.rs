use std::iter;
use std::ops;

// A polynomial over byte values.
#[derive(Debug, PartialEq, Clone)]
pub struct Polynomial {
    // Term coefficients for powers of x starting at 0 (i.e. coefficients[i] is for term cx^i).
    // The last element must always be non-zero. This allows us to efficiently compute the degree
    // from the length of this list without tracking it separately.
    coefficients: Vec<u8>,
}

impl ops::Add for &Polynomial {
    type Output = Polynomial;

    // Adds to polynomials together. This involves doing a pointwise sum of coefficients.
    fn add(self: Self, other: Self) -> Self::Output {
        let shorter;
        let longer;
        if self.coefficients.len() > other.coefficients.len() {
            shorter = &other.coefficients;
            longer = &self.coefficients;
        } else {
            shorter = &self.coefficients;
            longer = &other.coefficients;
        }

        // Using wrapping_add gives us addition modulo 256 (u8::max_value()).
        let new_coefficients: Vec<u8> = shorter
            .into_iter()
            .cloned()
            .chain(iter::repeat(0u8))
            .zip(longer)
            .map(|(x, y)| x.wrapping_add(*y))
            .collect();
        return Polynomial::from_coefficients(&new_coefficients);
    }
}

impl ops::Add for Polynomial {
    type Output = Polynomial;

    // Adds to polynomials together. This involves doing a pointwise sum of coefficients.
    fn add(self: Self, other: Self) -> Self::Output {
        &self + &other
    }
}

impl ops::Mul for &Polynomial {
    type Output = Polynomial;

    fn mul(self: Self, other: Self) -> Self::Output {
        if self.is_zero() || other.is_zero() {
            return Polynomial::zero();
        }

        // Compute the degree of the resulting polynomial as the sum of degrees
        let degree = self.degree() + other.degree();

        let mut new_coefficients: Vec<u8> = iter::repeat(0u8).take((degree + 1) as usize).collect();
        for (e1, c1) in self.coefficients.iter().enumerate() {
            for (e2, c2) in other.coefficients.iter().enumerate() {
                let e = e1.wrapping_add(e2);
                let c = c1.wrapping_mul(*c2);
                new_coefficients[e] = new_coefficients[e].wrapping_add(c);
            }
        }

        return Polynomial::from_coefficients(&new_coefficients);
    }
}

impl ops::Mul for Polynomial {
    type Output = Polynomial;

    fn mul(self: Self, other: Self) -> Self::Output {
        &self * &other
    }
}

impl Polynomial {
    // Returns the "zero" polynomial which is defined as the polynomial with no coefficients and
    // degree -1.
    pub fn zero() -> Self {
        Polynomial {
            coefficients: Vec::new(),
        }
    }

    // Creates a Polynomial from a given vector of coefficients. Has degree d == coefficients.len()
    // - 1.
    pub fn from_coefficients(coefficients: &[u8]) -> Self {
        Polynomial {
            coefficients: Vec::from(coefficients),
        }
    }

    // Computes a single term Polynomial P such that P(i) == values[i].
    pub fn single_term(values: &[u8], i: u8) -> Self {
        if values.len() == 0 {
            return Polynomial::zero();
        }

        let xi = i;
        let yi = values[i as usize];

        // Computes the term:
        //        ___
        //       |   | (x - xj)
        //  yi * |   | ---------
        //       |   | (xi - xj)
        //      j /= i
        let mut term = Self::from_coefficients(&[yi]);
        for (xj, _yj) in values.iter().enumerate().filter(|(x, _y)| *x as u8 != xi) {
            let xj = xj as u8;
            // Equivalent to the term:
            //
            //   (x - xj)
            //   ---------
            //   (xi - xj)
            let denominator = xi.wrapping_sub(xj);
            let neg = xj.wrapping_neg();
            let zeroth_term = xj.wrapping_neg().wrapping_div(denominator);
            let first_term = 1u8.wrapping_div(denominator);
            let p = Self::from_coefficients(&[zeroth_term, first_term]);
            println!("Constructing subterm xi: {}, xj: {}, neg: {}, denominator: {}, zeroth_term: {}, first_term: {}, p: {:?}", xi, xj, neg, denominator, zeroth_term, first_term, p.clone());

            term = term * p;
        }

        return term;
    }

    // Generates a polynomial from the given values. The values are intepreted as y-values for the
    // polynomial with the x-values being their index within the vector. That is to say, for a
    // vector of n values, we would interpolate using [(0, values[0], ..., (n-1, values[n-1])].
    pub fn interpolate(values: &[u8]) -> Self {
        if values.len() == 0 {
            return Self::zero();
        }
        return (0..values.len() - 1)
            .map(|i| Self::single_term(values, i as u8))
            .fold(Self::zero(), |x, y| &x + &y);
    }

    // Returns the degree of the Polynomial which is defined as -1 for the zero Polynomial and the
    // largest exponent (power) of x for any term (e.g. for `5 + x + 2x^3` it is `3`) otherwise,
    // with the constant term having exponent `0`.
    pub fn degree(self: &Self) -> i64 {
        return self.coefficients.len() as i64 - 1;
    }

    // Returns whether this Polynomial is the zero Polynomial.
    pub fn is_zero(self: &Self) -> bool {
        return self.degree() == -1;
    }

    pub fn evaluate(self: &Self, x: u8) -> u8 {
        let mut result: u8 = 0;
        for (e, c) in self.coefficients.iter().enumerate() {
            result = result.wrapping_add(x.wrapping_pow(e as u32).wrapping_mul(*c));
        }

        return result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degree_zero() {
        let p = Polynomial::zero();
        assert_eq!(p.degree(), -1);
    }

    #[test]
    fn degree_constant() {
        let p = Polynomial::from_coefficients(&[1]);
        assert_eq!(p.degree(), 0);
    }

    #[test]
    fn degree_linear() {
        let p = Polynomial::from_coefficients(&[0, 1]);
        assert_eq!(p.degree(), 1);
    }

    #[test]
    fn is_zero_true() {
        let p = Polynomial::from_coefficients(&[]);
        assert_eq!(p.is_zero(), true);
    }

    #[test]
    fn is_zero_false() {
        let p = Polynomial::from_coefficients(&[1]);
        assert_eq!(p.is_zero(), false);
    }

    #[test]
    fn evaluate_zero() {
        let p = Polynomial::zero();
        assert_eq!(p.evaluate(5), 0);
    }

    #[test]
    fn evaluate_constant() {
        let p = Polynomial::from_coefficients(&[7]);
        assert_eq!(p.evaluate(5), 7);
    }

    #[test]
    fn evaluate_linear() {
        let p = Polynomial::from_coefficients(&[7, 2]);
        assert_eq!(p.evaluate(5), 17);
    }

    #[test]
    fn evaluate_quadratic() {
        let p = Polynomial::from_coefficients(&[7, 2, 3]);
        assert_eq!(p.evaluate(5), 92);
    }

    #[test]
    fn evaluate_wrapping() {
        let p = Polynomial::from_coefficients(&[7, 2, 3, 4]);
        assert_eq!(p.evaluate(5), 80);
    }

    #[test]
    fn add_zero() {
        assert_eq!(Polynomial::zero() + Polynomial::zero(), Polynomial::zero());
    }

    #[test]
    fn add_zero_anything() {
        let zero = Polynomial::zero();
        let anything = Polynomial::from_coefficients(&[5, 8, 10]);
        assert_eq!(&zero + &anything, anything);
    }

    #[test]
    fn add_same_degree() {
        let first = Polynomial::from_coefficients(&[1, 2, 3]);
        let second = Polynomial::from_coefficients(&[5, 8, 10]);
        let expected = Polynomial::from_coefficients(&[6, 10, 13]);
        assert_eq!(&first + &second, expected);
    }

    #[test]
    fn add_different_degree() {
        let first = Polynomial::from_coefficients(&[1, 2, 3, 4, 5, 6, 7]);
        let second = Polynomial::from_coefficients(&[5, 0, 10]);
        let expected = Polynomial::from_coefficients(&[6, 2, 13, 4, 5, 6, 7]);
        assert_eq!(&first + &second, expected);
        // Also test for commutativity of addition.
        assert_eq!(&second + &first, expected);
    }

    #[test]
    fn add_wrapping() {
        let first = Polynomial::from_coefficients(&[255, 128, 255]);
        let second = Polynomial::from_coefficients(&[1, 128, 255]);
        let expected = Polynomial::from_coefficients(&[0, 0, 254]);
        assert_eq!(&first + &second, expected);
        // Also test for commutativity of addition.
        assert_eq!(&second + &first, expected);
    }

    #[test]
    fn mul_zero_anything() {
        assert_eq!(Polynomial::zero() * Polynomial::zero(), Polynomial::zero());
        assert_eq!(
            Polynomial::zero() * Polynomial::from_coefficients(&[1]),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::zero() * Polynomial::from_coefficients(&[1, 2, 3]),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::from_coefficients(&[1, 2, 3]) * Polynomial::zero(),
            Polynomial::zero()
        );
    }

    #[test]
    fn mul_constants() {
        let first = Polynomial::from_coefficients(&[3]);
        let second = Polynomial::from_coefficients(&[5]);
        let expected = Polynomial::from_coefficients(&[15]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn mul_constant_linear() {
        let first = Polynomial::from_coefficients(&[3]);
        let second = Polynomial::from_coefficients(&[5, 2]);
        let expected = Polynomial::from_coefficients(&[15, 6]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn mul_linear_linear() {
        let first = Polynomial::from_coefficients(&[3, 7]);
        let second = Polynomial::from_coefficients(&[5, 2]);
        let expected = Polynomial::from_coefficients(&[15, 41, 14]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn mul_quadratic_quadratic() {
        let first = Polynomial::from_coefficients(&[3, 7, 4]);
        let second = Polynomial::from_coefficients(&[5, 2, 6]);
        let expected = Polynomial::from_coefficients(&[15, 41, 52, 50, 24]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn mul_quadratic_quadratic_with_wrapping() {
        let first = Polynomial::from_coefficients(&[12, 7, 4]);
        let second = Polynomial::from_coefficients(&[5, 20, 6]);
        let expected = Polynomial::from_coefficients(&[60, 19, 232, 122, 24]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn mul_quadratic_quadratic_with_zeros() {
        let first = Polynomial::from_coefficients(&[3, 7, 4]);
        let second = Polynomial::from_coefficients(&[0, 0, 6]);
        let expected = Polynomial::from_coefficients(&[0, 0, 18, 42, 24]);
        assert_eq!(&first * &second, expected);
        assert_eq!(&second * &first, expected);
    }

    #[test]
    fn single_term_zero() {
        let p = Polynomial::single_term(&[], 0);
        assert_eq!(p, Polynomial::zero());
    }

    #[test]
    fn single_term_constant() {
        let p = Polynomial::single_term(&[5], 0);
        assert_eq!(p, Polynomial::from_coefficients(&[5]));
        assert_eq!(p.evaluate(0), 5);
        assert_eq!(p.evaluate(1), 5);
        assert_eq!(p.evaluate(2), 5);
    }

    #[test]
    fn single_term_linear() {
        let p0 = Polynomial::single_term(&[1, 2], 0);
        let p1 = Polynomial::single_term(&[1, 2], 1);
        assert_eq!(p0.evaluate(0), 1);
        assert_eq!(p1.evaluate(1), 2);
    }

    // #[test]
    // fn single_term_quadratic() {
    //     // let p0 = Polynomial::single_term(&[1, 2, 3], 0);
    //     let p1 = Polynomial::single_term(&[1, 2, 3], 1);
    //     // let p2 = Polynomial::single_term(&[1, 2, 3], 2);
    //     // assert_eq!(p0.evaluate(0), 1);
    //     assert_eq!(p1.evaluate(1), 2);
    //     // assert_eq!(p2.evaluate(2), 3);
    // }

    // #[test]
    // fn interpolate_linear() {
    //     let p = Polynomial::interpolate(&[1, 2]);
    //     assert_eq!(p, Polynomial::from_coefficients(&[1, 1]));
    //     assert_eq!(p.evaluate(0), 1);
    //     assert_eq!(p.evaluate(1), 2);
    //     assert_eq!(p.evaluate(2), 3);
    // }
}
