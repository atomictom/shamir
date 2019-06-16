use std::ops;

// A finite field with 256 elements. Also known as a Galois extension field, F(2^8). As a field, it
// supports addition, additive inverse, additive identity, multiplication for non-zero elements,
// multiplicative identity, and multiplicative inverse.
//
// A finite extension field is a "polynomial" where each coefficient is an element of the field
// being extended (in this case, our extension field is F(2^8) so our coefficients are elements
// of F(2). Because we are using F(2^8) we have a polynomial of length 8, each with elements
// either 0 or 1. Thus, we can represent each polynomial with an 8 bit integer.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FiniteField256(pub u8);

impl FiniteField256 {
    // Additive identity.
    pub fn zero() -> Self {
        FiniteField256(0u8)
    }

    // Multiplicative identity.
    pub fn one() -> Self {
        FiniteField256(1u8)
    }

    // Represents the polynomial "x" (aka 1*x^1 + 0).
    pub fn x() -> Self {
        FiniteField256(2u8)
    }

    // Returns the multiplicative inverse of an element.
    pub fn inv(self: &Self) -> Self {
        for i in 1..=255 {
            let x = Self::from_byte(i);
            if &x * self == Self::one() {
                return x;
            }
        }
        assert!(false, "No multiplicative inv for {:?}", self);
        return Self::zero();
    }

    pub fn pow(self: &Self, power: u32) -> Self {
        let mut result = Self::one();
        for _ in 0..power {
            result = &result * self;
        }
        return result;
    }

    pub fn from_byte(byte: u8) -> Self {
        FiniteField256(byte)
    }

    pub fn to_byte(self: &Self) -> u8 {
        let Self(byte) = *self;
        return byte;
    }
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

    fn add(self: Self, Self(b): Self) -> Self::Output {
        let Self(a) = self;
        Self(a ^ b)
    }
}

impl ops::Add for &FiniteField256 {
    type Output = FiniteField256;

    fn add(self: Self, other: Self) -> Self::Output {
        return self.clone() + other.clone();
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
        return self.clone() + other.clone();
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
        return self.clone();
    }
}

impl ops::Mul for FiniteField256 {
    type Output = FiniteField256;

    fn mul(self: Self, other: Self) -> Self::Output {
        // The AES polynomial, without the leading bit (we shift it out from `b` before reducing).
        let irreducible = 0b00011011u8;
        let Self(mut a) = self;
        let Self(mut b) = other;
        let mut result = 0u8;
        // "Russian peasant" multiplication for GF extension fields.
        for _ in 0..8 {
            // If (x & 1) == 1, then negating it gives all "1"s via 2s-complement, otherwise, -0 ==
            // 0, so we can use this to mask in/out certain values.
            result ^= (b & 1).wrapping_neg() & a;
            // Shift and break as early as possible.
            b >>= 1;
            if b == 0 {
                break;
            }
            // If b would have a "carry" when doubling it, reduce it via the irreducible
            // polynomial.
            a = (a << 1) ^ (((a & 0b10000000) >> 7).wrapping_neg() & irreducible);
        }
        FiniteField256(result)
    }
}

impl ops::Mul for &FiniteField256 {
    type Output = FiniteField256;

    fn mul(self: Self, other: Self) -> Self::Output {
        return self.clone() * other.clone();
    }
}

impl ops::Div for FiniteField256 {
    type Output = FiniteField256;

    fn div(self: Self, other: Self) -> Self::Output {
        return self * other.inv();
    }
}

impl ops::Div for &FiniteField256 {
    type Output = FiniteField256;

    fn div(self: Self, other: Self) -> Self::Output {
        return self.clone() / other.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_additive_identity() {
        for i in 0..=255 {
            let x = FiniteField256::from_byte(i);
            assert_eq!(x, &FiniteField256::zero() + &x);
            assert_eq!(x, &x + &FiniteField256::zero());
        }
    }

    #[test]
    fn element_is_own_inverse() {
        for i in 0..=255 {
            let x = FiniteField256::from_byte(i);
            assert_eq!(FiniteField256::zero(), &x + &x);
            assert_eq!(x, -&x);
        }
    }

    #[test]
    fn addition_is_same_as_subtraction() {
        let x = FiniteField256::from_byte(0x7C);
        let y = FiniteField256::from_byte(0xF1);
        assert_eq!(&x + &y, &x - &y);
    }

    #[test]
    fn one_multiplicative_identity() {
        for i in 0..=255 {
            let x = FiniteField256::from_byte(i);
            assert_eq!(x, &FiniteField256::one() * &x);
            assert_eq!(x, &x * &FiniteField256::one());
        }
    }

    #[test]
    fn mul_commutative() {
        for i in 0..=255 {
            for j in 0..=255 {
                let x = FiniteField256::from_byte(i);
                let y = FiniteField256::from_byte(j);
                assert_eq!(&x * &y, &y * &x);
            }
        }
    }

    #[test]
    fn inv_closed() {
        for i in 1..=255 {
            let x = FiniteField256::from_byte(i);
            assert!(x.inv() != FiniteField256::zero());
        }
    }

    #[test]
    fn inv_identity() {
        for i in 1..=255 {
            let x = FiniteField256::from_byte(i);
            assert_eq!(&x * &x.inv(), FiniteField256::one());
        }
    }

    #[test]
    fn mul_generator() {
        let mut exists: [bool; 256] = [false; 256];
        let x = FiniteField256::from_byte(3);
        for i in 1..=255 {
            let y = x.pow(i).to_byte();
            println!("y: {:01x}", y);
            exists[y as usize] = true;
        }
        for i in 1..=255 {
            println!("i: {:?}, exists: {:?}", i, exists[i]);
            assert!(exists[i]);
        }
    }
    //
    // #[test]
    // fn mul_div_inverse() {
    //     for i in 1..=255 {
    //         for j in 1..=255 {
    //             let x = FiniteField256::from_byte(i);
    //             let y = FiniteField256::from_byte(j);
    //             let z = &x * &y;
    //             assert_eq!(&z / &x, y);
    //             assert_eq!(&z / &y, x);
    //         }
    //     }
    // }
}
