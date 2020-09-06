use crate::finite_field::Field256;
use std::iter;

// A polynomial over byte values.
#[derive(Debug, PartialEq, Clone)]
pub struct Polynomial {
    // Term coefficients for powers of x starting at 0 (i.e. coefficients[i] is for term cx^i).
    // The last element must always be non-zero. This allows us to efficiently compute the degree
    // from the length of this list without tracking it separately.
    coefficients: Vec<u8>,
}

impl Polynomial {
    // Returns the "zero" polynomial which is defined as the polynomial with no coefficients and
    // degree -1.
    fn zero() -> Self {
        return Polynomial {
            coefficients: Vec::new(),
        };
    }

    // Creates a Polynomial from a given vector of coefficients. Has degree d == coefficients.len()
    // - 1.
    fn from_bytes(coefficients: &[u8]) -> Self {
        return Polynomial {
            // coefficients: Vec::from(coefficients),
            coefficients: Vec::from(coefficients),
        };
    }

    // Returns the degree of the Polynomial which is defined as -1 for the zero Polynomial and the
    // largest exponent (power) of x for any term (e.g. for `5 + x + 2x^3` it is `3`) otherwise,
    // with the constant term having exponent `0`.
    fn degree(self: &Self) -> i64 {
        return self.coefficients.len() as i64 - 1;
    }

    // Returns whether this Polynomial is the zero Polynomial.
    fn is_zero(self: &Self) -> bool {
        return self.degree() == -1;
    }

    // Adds to polynomials together. This involves doing a pointwise sum of coefficients.
    fn add<F: Field256>(&self, other: &Self) -> Self {
        let shorter;
        let longer;
        if self.coefficients.len() > other.coefficients.len() {
            shorter = &other.coefficients;
            longer = &self.coefficients;
        } else {
            shorter = &self.coefficients;
            longer = &other.coefficients;
        }

        let new_coefficients: Vec<_> = shorter
            .into_iter()
            .cloned()
            .chain(iter::repeat(F::zero()))
            .zip(longer)
            .map(|(x, y)| F::add(x, *y))
            .collect();
        return Polynomial::from_bytes(&new_coefficients);
    }

    fn mul<F: Field256>(self: Self, other: &Self, field: &F) -> Self {
        if self.is_zero() || other.is_zero() {
            return Polynomial::zero();
        }

        // Compute the degree of the resulting polynomial as the sum of degrees
        let degree = self.degree() + other.degree();

        let mut new_coefficients: Vec<_> = iter::repeat(F::zero())
            .take((degree + 1) as usize)
            .collect();
        for (e1, c1) in self.coefficients.iter().enumerate() {
            for (e2, c2) in other.coefficients.iter().enumerate() {
                let e: usize = e1 + e2;
                let c = field.mul(*c1, *c2);
                new_coefficients[e] = F::add(new_coefficients[e], c);
            }
        }

        return Polynomial::from_bytes(&new_coefficients);
    }

    // Computes a single term Polynomial P such that P(i) == values[i].
    fn single_term<F: Field256>(points: &[(u8, u8)], (xi, yi): (u8, u8), field: &F) -> Self {
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
            let xj = *xj;
            let denominator = F::sub(xi, xj);
            let zeroth_term = field.div(xj, denominator);
            let first_term = field.inv(denominator);
            let p = Self::from_bytes(&[zeroth_term, first_term]);
            // println!("Constructing subterm xi: {:?}, xj: {:?}, denominator: {:?}, zeroth_term: {:?}, first_term: {:?}, p: {:?}", xi, xj, denominator, zeroth_term, first_term, p.clone());

            term = term.mul(&p, field);
        }

        return term;
    }

    #[allow(unused)]
    fn single_term_ys<F: Field256>(ys: &[u8], i: u8, field: &F) -> Self {
        assert!((i as usize) < ys.len());
        let points: Vec<_> = ys.iter().enumerate().map(|(x, y)| (x as u8, *y)).collect();
        Self::single_term(&points[..], (i, ys[i as usize]), field)
    }

    // Generates a polynomial from the given values. The values are (x, y) coordinate pairs.
    pub fn interpolate_points<F: Field256>(points: &[(u8, u8)], field: &F) -> Self {
        if points.len() == 0 {
            return Self::zero();
        }
        assert!(points.len() < 256);
        return points
            .iter()
            .map(|p| Self::single_term(points, *p, field))
            .fold(Self::zero(), |x, y| x.add::<F>(&y));
    }

    // Generates a polynomial from the given values. The values are intepreted as y-values for the
    // polynomial with the x-values being their index within the vector. That is to say, for a
    // vector of n values, we would interpolate using [(0, values[0], ..., (n-1, values[n-1])].
    pub fn interpolate<F: Field256>(ys: &[u8], field: &F) -> Self {
        let points: Vec<_> = ys.iter().enumerate().map(|(x, y)| (x as u8, *y)).collect();
        Self::interpolate_points(&points[..], field)
    }

    pub fn evaluate<F: Field256>(self: &Self, x: u8, field: &F) -> u8 {
        let mut result: u8 = F::zero();
        for (e, c) in self.coefficients.iter().enumerate() {
            result = F::add(result, field.mul(field.exp(x, e as u8), *c));
        }

        return result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finite_field::DirectField;

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
        let direct = DirectField::default();
        assert_eq!(p.evaluate(5, &direct), 0);
    }

    #[test]
    fn add_zero() {
        assert_eq!(
            Polynomial::zero().add::<DirectField>(&Polynomial::zero()),
            Polynomial::zero()
        );
    }

    #[test]
    fn add_zero_anything() {
        let zero = Polynomial::zero();
        let anything = Polynomial::from_bytes(&[5, 8, 10]);
        assert_eq!(zero.add::<DirectField>(&anything), anything);
    }

    #[test]
    fn mul_zero_anything() {
        let direct = DirectField::default();
        assert_eq!(
            Polynomial::zero().mul(&Polynomial::zero(), &direct),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::zero().mul(&Polynomial::from_bytes(&[1]), &direct),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::zero().mul(&Polynomial::from_bytes(&[1, 2, 3]), &direct),
            Polynomial::zero()
        );
        assert_eq!(
            Polynomial::from_bytes(&[1, 2, 3]).mul(&Polynomial::zero(), &direct),
            Polynomial::zero()
        );
    }

    #[test]
    fn single_term_constant() {
        let direct = DirectField::default();
        let p = Polynomial::single_term_ys(&[5], 0, &direct);
        assert_eq!(p, Polynomial::from_bytes(&[5]));
        assert_eq!(p.evaluate(0, &direct), 5);
        assert_eq!(p.evaluate(1, &direct), 5);
        assert_eq!(p.evaluate(2, &direct), 5);
    }

    #[test]
    fn single_term_linear() {
        let direct = DirectField::default();
        let p0 = Polynomial::single_term_ys(&[1, 2], 0, &direct);
        let p1 = Polynomial::single_term_ys(&[1, 2], 1, &direct);
        assert_eq!(p0.evaluate(0, &direct), 1);
        assert_eq!(p1.evaluate(1, &direct), 2);
    }

    #[test]
    fn interpolate_same() {
        let direct = DirectField::default();
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF], &direct);
        let p1 =
            Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (3, 0xEF)], &direct);
        assert_eq!(p0, p1);
    }

    #[test]
    fn evaluate_interpolated_initial_gives_initial() {
        let direct = DirectField::default();
        let p = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF], &direct);
        assert_eq!(0xDE, p.evaluate(0, &direct));
        assert_eq!(0xAD, p.evaluate(1, &direct));
        assert_eq!(0xBE, p.evaluate(2, &direct));
        assert_eq!(0xEF, p.evaluate(3, &direct));
    }

    #[test]
    fn evaluate_interpolated_after() {
        let direct = DirectField::default();
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF], &direct);
        let p1 =
            Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (3, 0xEF)], &direct);
        assert_eq!(p0.evaluate(4, &direct), p1.evaluate(4, &direct));
    }

    #[test]
    fn evaluate_forget_evaluate() {
        let direct = DirectField::default();
        let p0 = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF], &direct);
        let e = p0.evaluate(4, &direct);
        let p1 =
            Polynomial::interpolate_points(&[(0, 0xDE), (1, 0xAD), (2, 0xBE), (4, e)], &direct);
        assert_eq!(p0, p1);
    }

    #[test]
    fn evaluate_forget_more_evaluate() {
        let direct = DirectField::default();
        let p = Polynomial::interpolate(&[0xDE, 0xAD, 0xBE, 0xEF], &direct);
        let points: Vec<_> = (4..8).map(|x| (x, p.evaluate(x, &direct))).collect();
        assert_eq!(p, Polynomial::interpolate_points(&points, &direct));
    }
}
