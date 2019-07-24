// The AES polynomial, without the leading bit (we shift it out before reducing).
const IRREDUCIBLE: u8 = 0b00011011;

// An element of GF(2^8) which, when raised to powers 0..255, generates every element of the field.
const GENERATOR: u8 = 0b11;

// A finite field with 256 elements. Also known as a Galois extension field, GF(2^8). As a field,
// it supports addition, additive inverse (and thus subtraction), additive identity, multiplication
// for non-zero elements, multiplicative identity, and multiplicative inverse (and thus division).
//
// A finite extension field is a "polynomial" where each coefficient is an element of the field
// being extended (in this case, our extension field is GF(2^8) so our coefficients are elements of
// GF(2). Because we are using GF(2^8) we have a polynomial of length 8, each with elements either
// 0 or 1. Thus, we can represent each polynomial with an 8 bit integer.
//
// The only function that must be implemented is mul(), but others can be implemented for speed.
trait Field256 {
    // Additive identity.
    fn zero() -> u8 {
        return 0;
    }
    // Multiplicative identity.
    fn one() -> u8 {
        return 1;
    }

    // Addition and subtraction in a finite extension field (that is, GF(x^n) where n>1), which
    // treats elements as polynomials, are defined as polynomial addition/subtraction using the
    // underlying field's addition/subtraction element for operations on the coefficients (that is,
    // GF(x)). In the case of GF(2^8), because each coefficient is represented by a bit, we do
    // bitwise addition or subtraction (that means we do addition/subtraction without carry because
    // we do it on each coefficient independently).  However, addition and subtraction in the
    // underlying field, F(2), are equivalent to the XOR operation (e.g. 1+1 `mod` 2 == 0). Thus,
    // both addition and subtraction of any field GF(2^n) (GF(2^8) in our case) can be defined as
    // the bitwise-XOR of the two (bitvector) polynomials. Because addition and subtraction are the
    // same operation in GF(2^n), this also means that every element of the extension field is its
    // own additive inverse.
    fn add(x: u8, y: u8) -> u8 {
        return x ^ y;
    }
    fn sub(x: u8, y: u8) -> u8 {
        return x ^ y;
    }
    fn neg(x: u8) -> u8 {
        return x;
    }

    // May use self to speed up the computation.
    fn mul(&self, x: u8, y: u8) -> u8;
    fn div(&self, x: u8, y: u8) -> u8 {
        return self.mul(x, self.inv(y));
    }

    // Returns x ^ y. May use self to speed up the computation.
    fn exp(&self, x: u8, y: u8) -> u8 {
        let mut result = Self::one();
        for _ in 0..y {
            result = self.mul(result, x);
        }
        return result;
    }

    // Returns the multiplicative inverse of an element. For simplicity it's just a brute-force
    // search of all 255 non-zero elements to find which, when multiplied by the input element,
    // gives 1 (and thus, by definition, is the inverse).
    fn inv(&self, x: u8) -> u8 {
        for i in 1..=255 {
            if self.mul(i, x) == Self::one() {
                return i;
            }
        }
        assert!(false, "No multiplicative inv for {:?}", x);
        return Self::zero();
    }
}

// Field implementation that does computations directly.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct DirectField;

impl Field256 for DirectField {
    fn mul(&self, x: u8, y: u8) -> u8 {
        let mut result = Self::zero();
        let mut a = x;
        let mut b = y;
        // "Russian peasant" multiplication for GF extension fields.
        for _ in 0..8 {
            // If (a & 1) == 1, then negating it gives all "1"s via 2s-complement, otherwise, -0 ==
            // 0, so we can use this to mask in/out certain values.
            result ^= (b & 1).wrapping_neg() & a;
            // Shift and break as early as possible.
            b >>= 1;
            if b == 0 {
                break;
            }
            // If b would have a "carry" when doubling it, reduce it via the irreducible
            // polynomial.
            a = (a << 1) ^ (((a & 0b10000000) >> 7).wrapping_neg() & IRREDUCIBLE);
        }
        return result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_additive_identity() {
        for i in 0..=255 {
            assert_eq!(i, DirectField::add(i, DirectField::zero()));
            assert_eq!(i, DirectField::add(DirectField::zero(), i));
        }
    }

    #[test]
    fn element_is_own_inverse() {
        for i in 0..=255 {
            assert_eq!(DirectField::zero(), DirectField::add(i, i));
            assert_eq!(i, DirectField::neg(i));
        }
    }

    #[test]
    fn addition_is_same_as_subtraction() {
        let x = 0x7C;
        let y = 0xF1;
        assert_eq!(DirectField::add(x, y), DirectField::sub(x, y));
    }

    #[test]
    fn one_multiplicative_identity() {
        let field = DirectField::default();
        for i in 0..=255 {
            assert_eq!(i, field.mul(DirectField::one(), i));
            assert_eq!(i, field.mul(i, DirectField::one()));
        }
    }

    #[test]
    fn mul_commutative() {
        let field = DirectField::default();
        for i in 0..=255 {
            for j in 0..=255 {
                assert_eq!(field.mul(i, j), field.mul(j, i));
            }
        }
    }

    #[test]
    fn inv_closed() {
        let field = DirectField::default();
        for i in 1..=255 {
            assert!(field.inv(i) != DirectField::zero());
        }
    }

    #[test]
    fn inv_identity() {
        let field = DirectField::default();
        for i in 1..=255 {
            assert_eq!(field.mul(i, field.inv(i)), DirectField::one());
        }
    }

    #[test]
    fn mul_generator() {
        let field = DirectField::default();
        let mut exists: [bool; 256] = [false; 256];
        for i in 1..=255 {
            let x = field.exp(GENERATOR, i);
            println!("x: {:01x}", x);
            exists[x as usize] = true;
        }
        for i in 1..=255 {
            println!("i: {:?}, exists: {:?}", i, exists[i]);
            assert!(exists[i]);
        }
    }

    #[test]
    fn mul_div_inverse() {
        let field = DirectField::default();
        for i in 1..=255 {
            for j in 1..=255 {
                let z = field.mul(i, j);
                assert_eq!(field.div(z, i), j);
                assert_eq!(field.div(z, j), i);
            }
        }
    }
}
