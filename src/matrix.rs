use std::iter;

pub struct Matrix {
    mat: Vec<Vec<u8>>,
}

impl Matrix {
    pub fn zero(r: usize, c: usize) -> Matrix {
        let row: Vec<u8> = iter::repeat(0).take(c).collect();
        let mat: Vec<Vec<u8>> = iter::repeat(row).take(r).collect();
        return Matrix { mat: mat };
    }

    pub fn identity(n: usize) -> Matrix {
        let row: Vec<u8> = iter::repeat(0).take(n).collect();
        let mut mat: Vec<Vec<u8>> = iter::repeat(row).take(n).collect();
        for i in 0..n {
            mat[i][i] = 1;
        }
        return Matrix { mat: mat };
    }

    pub fn mul(self: &Self, other: &Self) -> Matrix {
        assert!(self.mat.len() == other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero() {
        let zero = Matrix::zero(10, 5);
        assert_eq!(zero.mat.len(), 10);
        assert_eq!(zero.mat[0].len(), 5);
    }

    #[test]
    fn identity() {
        let id = Matrix::identity(5);
        assert_eq!(id.mat.len(), 5);
        assert_eq!(id.mat[0].len(), 5);
        assert_eq!(id.mat[0][0], 1);
        assert_eq!(id.mat[0][1], 0);
        assert_eq!(id.mat[1][0], 0);
    }
}
