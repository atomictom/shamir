#![feature(no_panic_pow)]
use std::iter;
use std::ops;

// A polynomial over byte values.
pub struct Polynomial {
    // Term coefficients for powers of x starting at 0 (i.e. coefficients[i] is for term cx^i).
    // The last element must always be non-zero. This allows us to efficiently compute the degree
    // from the length of this list without tracking it separately.
    coefficients: Vec<u8>,
}

impl ops::Add for Polynomial {
    type Output = Self;

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
        return Self::from_coefficients(&new_coefficients);
    }
}

impl ops::Mul for Polynomial {
    type Output = Self;

    fn mul(self: Self, other: Self) -> Self::Output {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }

        // Compute the degree of the resulting polynomial as the sum of degrees
        let degree = self.degree() + other.degree();

        let mut new_coefficients: Vec<u8> = iter::repeat(0u8).take((degree + 1) as usize).collect();
        for (e1, c1) in self.coefficients.iter().enumerate() {
            for (e2, c2) in other.coefficients.iter().enumerate() {
                new_coefficients[e1 + e2] += c1 * c2;
            }
        }

        return Self::from_coefficients(&new_coefficients);
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
        let xi = i;
        let yi = values[i as usize];

        // Computes the term:
        //        ___
        //       |   | (x - xj)
        //  yi * |   | ---------
        //       |   | (xi - xj)
        //      j /= i
        let mut term = Self::from_coefficients(&vec![yi]);
        for (xj, _yj) in values.iter().enumerate().filter(|(x, _y)| *x as u8 != xi) {
            let xj = xj as u8;
            // Equivalent to the term:
            //
            //   (x - xj)
            //   ---------
            //   (xi - xj)
            let denominator = xi.wrapping_sub(xj);
            let zeroth_term = xj.wrapping_neg().wrapping_div(denominator);
            let first_term = 1u8.wrapping_div(denominator);

            term = term * Self::from_coefficients(&vec![zeroth_term, first_term]);
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
            .fold(Self::zero(), |x, y| x + y);
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
