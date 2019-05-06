use std::ops;

// A finite field with 256 elements. Also known as a Galois extension field, F(2^8). As a field, it
// supports addition, additive inverse, additive identity, multiplication for non-zero elements,
// multiplicative identity, and multiplicative inverse.
#[derive(Debug, PartialEq, Eq, Clone, Send)]
pub struct FiniteField256 {
    // A finite extension field is a "polynomial" where each coefficient is an element of the field
    // being extended (in this case, our extension field is F(2^8) so our coefficients are elements
    // of F(2). Because we are using F(2^8) we have a polynomial of length 8, each with elements
    // either 0 or 1. Thus, we can represent each polynomial with an 8 bit integer. However, we
    // also need to support overflow during multiplication (the result of which will eventually be
    // reduced to an 8-bit integer by "modding" by an irreducible polynomial). So, we use a 16 bit
    // integer to support overflow.
    polynomial: i16,
}

// Addition and subtraction in a finite extension field are defined as standard polynomial
// addition/subtraction of the underlying polynomials using the underlying field's
// addition/subtraction element for each operation. In the case of F(2^8), because each coefficient
// is represented by a bit, we do bitwise addition or subtraction (that means we do
// addition/subtraction without carry because we do it on each coefficient independently). However,
// addition and subtraction in the underlying field, F(2), are equivalent to the XOR operation
// (e.g. 1+1 `mod` 2 == 0). Thus, both addition and subtraction of any field F(2^p) where p is any
// power (F(2^8) in our case) where the field extension polynomial coefficients are represented by
// bits can be defined as the bitwise-XOR of the two "polynomials". This also means that every
// element of the extension field is its own additive inverse.
impl ops::Add for FiniteField256 {
    type Output = FiniteField256;

    // Precondition: the upper 8-bits should be 0.
    // Postcondition: the upper 8-bits will be 0 if the precondition is met.
    fn add(self: Self, other: Self) -> Self::Output {
        return self.polynomial ^ other.polynomial;
    }
}

impl ops::Add for &FiniteField256 {
    type Output = FiniteField256;

    fn add(self: Self, other: Self) -> Self::Output {
        return *self + *other;
    }
}

impl ops::Sub for FiniteField256 {
    type Output = FiniteField256;

    fn sub(self: Self, other: Self) -> Self::Output {
        return self + other;
    }
}

impl ops::Sub for &FiniteField256 {
    type Output = FiniteField256;

    fn sub(self: Self, other: Self) -> Self::Output {
        return *self + *other;
    }
}

impl ops::Neg for FiniteField256 {
    type Output = FiniteField256;

    fn neg(self: Self) -> Self::Output {
        return self;
    }
}

impl ops::Neg for &FiniteField256 {
    type Output = FiniteField256;

    fn neg(self: Self) -> Self::Output {
        return *self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {}
}
