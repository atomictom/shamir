use crate::finite_field::DirectField;
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
        let new_coefficients: Vec<_> = shorter
            .into_iter()
            .cloned()
            .chain(iter::repeat(DirectField::zero()))
            .zip(longer)
            .map(|(x, y)| x + *y)
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

        let mut new_coefficients: Vec<_> = iter::repeat(FiniteField256::zero())
            .take((degree + 1) as usize)
            .collect();
        for (e1, c1) in self.coefficients.iter().enumerate() {
            for (e2, c2) in other.coefficients.iter().enumerate() {
                let e = e1 + e2;
                let c = c1 * c2;
                new_coefficients[e] = new_coefficients[e] + c;
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
    pub fn from_bytes(coefficients: &[u8]) -> Self {
        Polynomial {
            // coefficients: Vec::from(coefficients),
            coefficients: coefficients
                .iter()
                .map(|x| FiniteField256::from_byte(*x))
                .collect(),
        }
    }

    // Computes a single term Polynomial P such that P(i) == values[i].
    pub fn single_term(points: &[(u8, u8)], (xi, yi): (u8, u8)) -> Self {
        if points.len() == 0 {
            return Polynomial::zero();
        }

        // Computes the term:
        //        ___
        //       |   | (x - xj)
        //  yi * |   | ---------
        //       |   | (xi - xj)
        //      j /= i
        let mut term = Self::from_bytes(&[yi]);
        for (xj, _) in points.iter().filter(|(x, _)| *x != xi) {
            // Equivalent to the term:
            //
            //   (x - xj)
            //   ---------
            //   (xi - xj)
            let xj = FiniteField256::from_byte(*xj);
            let xi = FiniteField256::from_byte(xi);
            let denominator = xi - xj;
            let zeroth_term = xj / denominator;
            let first_term = FiniteField256::one() / denominator;
            let p = Self::from_coefficients(&[zeroth_term, first_term]);
            // println!("Constructing subterm xi: {:?}, xj: {:?}, denominator: {:?}, zeroth_term: {:?}, first_term: {:?}, p: {:?}", xi, xj, denominator, zeroth_term, first_term, p.clone());

            term = term * p;
        }

        return term;
    }

    fn single_term_ys(ys: &[u8], i: u8) -> Self {
        assert!((i as usize) < ys.len());
        let points: Vec<_> = ys.iter().enumerate().map(|(x, y)| (x as u8, *y)).collect();
        Self::single_term(&points[..], (i, ys[i as usize]))
    }

    // Generates a polynomial from the given values. The values are intepreted as y-values for the
    // polynomial with the x-values being their index within the vector. That is to say, for a
    // vector of n values, we would interpolate using [(0, values[0], ..., (n-1, values[n-1])].
    pub fn interpolate(ys: &[u8]) -> Self {
        let points: Vec<_> = ys.iter().enumerate().map(|(x, y)| (x as u8, *y)).collect();
        Self::interpolate_points(&points[..])
    }

    // Generates a polynomial from the given values. The values are (x, y) coordinate pairs.
    pub fn interpolate_points(points: &[(u8, u8)]) -> Self {
        if points.len() == 0 {
            return Self::zero();
        }
        assert!(points.len() < 256);
        return points
            .iter()
            .map(|p| Self::single_term(points, *p))
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
        let mut result: FiniteField256 = FiniteField256::zero();
        for (e, c) in self.coefficients.iter().enumerate() {
            result = result + (FiniteField256::from_byte(x).pow(e as u8) * *c);
        }

        return result.to_byte();
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
        let p = Polynomial::from_bytes(&[1]);
        assert_eq!(p.degree(), 0);
    }

    #[test]
    fn degree_linear() {
        let p = Polynomial::from_bytes(&[0, 1]);
        assert_eq!(p.degree(), 1);
    }

    #[test]
    fn is_zero_true() {
        let p = Polynomial::from_bytes(&[]);
        assert_eq!(p.is_zero(), true);
    }

    #[test]
    fn is_zero_false() {
        let p = Polynomial::from_bytes(&[1]);
        assert_eq!(p.is_zero(), false);
    }

    #[test]
    fn evaluate_zero() {
        let p = Polynomial::zero();
        assert_eq!(p.evaluate(5), 0);
    }

    #[test]
    fn add_zero() {
        assert_eq!(Polynomial::zero() + Polynomial::zero(), Polynomial::zero());
    }

    #[test]
    fn add_zero_anything() {
        let zero = Polynomial::zero();
        let anything = Polynomial::from_bytes(&[5, 8, 10]);
        assert_eq!(&zero + &anything, anything);
    }

    #[test]
    fn mul_zero_anything() {
        assert_eq!(Polynomial::zero() * Polynomial::zero(), Polynomial::zero());
        assert_eq!(
            Polynomial::zero() * Polynomial::from_bytes(&[1]),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::zero() * Polynomial::from_bytes(&[1, 2, 3]),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::from_bytes(&[1, 2, 3]) * Polynomial::zero(),
            Polynomial::zero()
        );
    }

    #[test]
    fn single_term_constant() {
        let p = Polynomial::single_term_ys(&[5], 0);
        assert_eq!(p, Polynomial::from_bytes(&[5]));
        assert_eq!(p.evaluate(0), 5);
        assert_eq!(p.evaluate(1), 5);
        assert_eq!(p.evaluate(2), 5);
    }

    #[test]
    fn single_term_linear() {
        let p0 = Polynomial::single_term_ys(&[1, 2], 0);
        let p1 = Polynomial::single_term_ys(&[1, 2], 1);
        assert_eq!(p0.evaluate(0), 1);
        assert_eq!(p1.evaluate(1), 2);
    }

    #[test]
    fn interpolate_same() {
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let p1 = Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (3, 0xEF)]);
        assert_eq!(p0, p1);
    }

    #[test]
    fn evaluate_interpolated_initial_gives_initial() {
        let p = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(0xDE, p.evaluate(0));
        assert_eq!(0xAD, p.evaluate(1));
        assert_eq!(0xBE, p.evaluate(2));
        assert_eq!(0xEF, p.evaluate(3));
    }

    #[test]
    fn evaluate_interpolated_after() {
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let p1 = Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (3, 0xEF)]);
        assert_eq!(p0.evaluate(4), p1.evaluate(4));
    }

    #[test]
    fn evaluate_forget_evaluate() {
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let e = p0.evaluate(4);
        let p1 = Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (4, e)]);
        assert_eq!(p0, p1);
    }

    #[test]
    fn evaluate_forget_more_evaluate() {
        let p = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let points: Vec<_> = (4..8).map(|x| (x, p.evaluate(x))).collect();
        assert_eq!(p, Polynomial::interpolate_points(&points));
    }
}
