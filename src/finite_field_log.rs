use crate::finite_field::FiniteField256;
use std::default;

pub struct FiniteField256Log {
    exp: [u8; 512],
    log: [u8; 512],
}

impl default::Default for FiniteField256Log {
    fn default() -> FiniteField256Log {
        let gen = FiniteField256::from_byte(3);
        let mut x = FiniteField256::one();
        let mut res = FiniteField256Log {
            exp: [0; 512],
            log: [0; 512],
        };
        for i in 0..255 {
            res.exp[i] = x.to_byte();
            res.exp[i + 255] = x.to_byte();
            res.log[x.to_byte() as usize] = i as u8;
            res.log[x.to_byte() as usize + 255] = i as u8;
            x = x * gen;
        }

        return res;
    }
}

impl FiniteField256Log {
    pub fn add(self: Self, x: u8, y: u8) -> u8 {
        return x ^ y;
    }

    pub fn add(self: Self, x: u8, y: u8) -> u8 {
        return x ^ y;
    }

    pub fn sub(self: Self, x: u8, y: u8) -> u8 {
        return x ^ y;
    }

    pub fn mul(self: Self, x: u8, y: u8) -> u8 {
        return self.exp[(self.log[x as usize] + self.log[y as usize]) as usize];
    }

    pub fn div(self: Self, x: u8, y: u8) -> u8 {
        return self.exp[(self.log[x as usize] - self.log[y as usize]) as usize + 256];
    }

    pub fn exp(self: Self, x: u8, y: u8) -> u8 {
        return self.exp[y as usize];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_additive_identity() {
        let field = FiniteField256Log::default();
        for i in 0..=255 {
            assert_eq!(i, field.add(i, 0));
            assert_eq!(i, field.add(0, i));
        }
    }

    #[test]
    fn element_is_own_inverse() {
        let field = FiniteField256Log::default();
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

    #[test]
    fn mul_div_inverse() {
        for i in 1..=255 {
            for j in 1..=255 {
                let x = FiniteField256::from_byte(i);
                let y = FiniteField256::from_byte(j);
                let z = &x * &y;
                assert_eq!(&z / &x, y);
                assert_eq!(&z / &y, x);
            }
        }
    }
}
