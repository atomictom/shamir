use std::default;

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
pub trait Field256 {
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
    // TODO: Use CLMUL or similar intrinsics with std::arch.
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

pub struct ExpLogField {
    exp: [u8; 512],
    log: [u8; 256],
}

impl default::Default for ExpLogField {
    fn default() -> Self {
        // TODO: Consider using e.g. lazy_static! to initialize the tables once and have all
        // implementations refer to them.
        let direct = DirectField::default();
        let mut x = Self::one();
        let mut res = Self {
            exp: [0; 512],
            log: [0; 256],
        };
        for i in 0..=255 {
            res.exp[i] = x;
            res.exp[i + 255] = x;
            res.log[x as usize] = i as u8;
            x = direct.mul(x, GENERATOR);
        }

        return res;
    }
}

impl Field256 for ExpLogField {
    fn mul(&self, x: u8, y: u8) -> u8 {
        if x == 0 || y == 0 {
            return 0;
        }
        let logx: i16 = self.log[x as usize] as i16;
        let logy: i16 = self.log[y as usize] as i16;
        return self.exp[(logx + logy) as usize];
    }

    fn div(&self, x: u8, y: u8) -> u8 {
        if x == 0 {
            return 0;
        } else if y == 0 {
            panic!("Cannot divide by zero!");
        }
        let logx: i16 = self.log[x as usize] as i16;
        let logy: i16 = self.log[y as usize] as i16;
        return self.exp[(logx - logy + 255) as usize];
    }

    fn inv(&self, x: u8) -> u8 {
        if x == 0 {
            return 0;
        }
        return self.exp[255 - self.log[x as usize] as usize];
    }

    fn exp(&self, x: u8, y: u8) -> u8 {
        if x == 0 {
            return 0;
        } else if y == 0 {
            return 1;
        }
        let logx: u16 = self.log[x as usize] as u16;
        let logy: u16 = self.log[y as usize] as u16;
        return self.exp[((logx * logy) % 256) as usize];
    }
}

pub struct TableField {
    inv: [u8; 256],
    mul: [[u8; 256]; 256],
}

impl default::Default for TableField {
    fn default() -> Self {
        // TODO: Consider using e.g. lazy_static! to initialize the tables once and have all
        // implementations refer to them.
        let direct = DirectField::default();
        let mut res = Self {
            inv: [0; 256],
            mul: [[0; 256]; 256],
        };
        for i in 1..=255 {
            res.inv[i as usize] = direct.inv(i)
        }
        for i in 0..=255 {
            for j in 0..=255 {
                res.mul[i as usize][j as usize] = direct.mul(i, j);
            }
        }

        return res;
    }
}

impl Field256 for TableField {
    fn mul(&self, x: u8, y: u8) -> u8 {
        if x == 0 || y == 0 {
            return 0;
        }
        return self.mul[x as usize][y as usize];
    }

    fn div(&self, x: u8, y: u8) -> u8 {
        if x == 0 {
            return 0;
        } else if y == 0 {
            panic!("Cannot divide by zero!");
        }
        return self.mul[x as usize][self.inv[y as usize] as usize];
    }

    fn inv(&self, x: u8) -> u8 {
        if x == 0 {
            return 0;
        }
        return self.inv[x as usize];
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

    fn one_multiplicative_identity_for<T: Field256 + Default>() {
        let field = T::default();
        for i in 0..=255 {
            assert_eq!(i, field.mul(T::one(), i));
            assert_eq!(i, field.mul(i, T::one()));
        }
    }

    #[test]
    fn one_multiplicative_identity() {
        one_multiplicative_identity_for::<DirectField>();
        one_multiplicative_identity_for::<ExpLogField>();
        one_multiplicative_identity_for::<TableField>();
    }

    fn mul_commutative_for<T: Field256 + Default>() {
        let field = T::default();
        for i in 0..=255 {
            for j in 0..=255 {
                assert_eq!(field.mul(i, j), field.mul(j, i));
            }
        }
    }

    #[test]
    fn mul_commutative() {
        mul_commutative_for::<DirectField>();
        mul_commutative_for::<ExpLogField>();
        mul_commutative_for::<TableField>();
    }

    fn inv_closed_for<T: Field256 + Default>() {
        let field = T::default();
        for i in 1..=255 {
            assert!(field.inv(i) != T::zero());
        }
    }

    #[test]
    fn inv_closed() {
        inv_closed_for::<DirectField>();
        inv_closed_for::<ExpLogField>();
        inv_closed_for::<TableField>();
    }

    fn inv_identity_for<T: Field256 + Default>() {
        let field = T::default();
        for i in 1..=255 {
            assert_eq!(field.mul(i, field.inv(i)), T::one());
        }
    }

    #[test]
    fn inv_identity() {
        inv_identity_for::<DirectField>();
        inv_identity_for::<ExpLogField>();
        inv_identity_for::<TableField>();
    }

    fn mul_generator_for<T: Field256 + Default>() {
        let field = T::default();
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
    fn mul_generator() {
        mul_generator_for::<DirectField>();
        mul_generator_for::<ExpLogField>();
        mul_generator_for::<TableField>();
    }

    fn mul_div_inverse_for<T: Field256 + Default>() {
        let field = T::default();
        for i in 1..=255 {
            for j in 1..=255 {
                let z = field.mul(i, j);
                println!("{:?} * {:?} = {:?}", i, j, z);
                println!(
                    "Expected: {:?} / {:?} = {:?}, Actual: = {:?}",
                    z,
                    i,
                    j,
                    field.div(z, i)
                );
                assert_eq!(field.div(z, i), j);
                println!(
                    "Expected: {:?} / {:?} = {:?}, Actual: = {:?}",
                    z,
                    i,
                    j,
                    field.div(z, j)
                );
                assert_eq!(field.div(z, j), i);
            }
        }
    }

    #[test]
    #[ignore]
    fn mul_div_inverse_direct_field() {
        mul_div_inverse_for::<DirectField>();
    }

    #[test]
    fn mul_div_inverse_exp_log_field() {
        mul_div_inverse_for::<ExpLogField>();
    }

    #[test]
    fn mul_div_inverse_table_field() {
        mul_div_inverse_for::<TableField>();
    }
}
