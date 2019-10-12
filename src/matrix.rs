use std::convert::TryFrom;
use std::iter;

#[derive(Debug, PartialEq, Eq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    mat: Vec<Vec<u8>>,
}

impl TryFrom<&[&[u8]]> for Matrix {
    type Error = &'static str;

    fn try_from(elems: &[&[u8]]) -> Result<Self, Self::Error> {
        let rows = elems.len();
        if rows <= 0 {
            return Err("Cannot have a matrix with 0 rows");
        }
        let cols = elems[0].len();
        if cols <= 0 {
            return Err("Cannot have a matrix with 0 cols");
        }

        let mut res = Matrix::zero(rows, cols);

        for (i, r) in elems.iter().enumerate() {
            for (j, c) in r.iter().enumerate() {
                res.mat[i][j] = *c;
            }
        }

        return Ok(res);
    }
}

impl Matrix {
    pub fn zero(rows: usize, cols: usize) -> Matrix {
        let row: Vec<u8> = iter::repeat(0).take(cols).collect();
        let mat: Vec<Vec<u8>> = iter::repeat(row).take(rows).collect();
        return Matrix {
            rows: rows,
            cols: cols,
            mat: mat,
        };
    }

    pub fn identity(n: usize) -> Matrix {
        let row: Vec<u8> = iter::repeat(0).take(n).collect();
        let mut mat: Vec<Vec<u8>> = iter::repeat(row).take(n).collect();
        for i in 0..n {
            mat[i][i] = 1;
        }
        return Matrix {
            rows: n,
            cols: n,
            mat: mat,
        };
    }

    pub fn mul(self: &Self, other: &Self) -> Matrix {
        assert!(self.cols == other.rows);
        let mut res = Matrix::zero(self.rows, other.cols);
        // Set each element of the matrix
        for i in 0..res.rows {
            for j in 0..res.cols {
                // Calculate a matrix element
                for k in 0..self.cols {
                    res.mat[i][j] += self.mat[i][k] * other.mat[k][j]
                }
            }
        }

        return res;
    }

    fn swap_row(self: &mut Self, from_row: usize, to_row: usize) -> &mut Self {
        let (mut x, mut y) = (&self.mat[to_row], &self.mat[from_row]);
        std::mem::swap(&mut x, &mut y);
        return self;
    }

    fn scale_row(self: &mut Self, row: usize, scale: u8) -> &mut Self {
        return self;
    }

    fn add_scaled_row(self: &mut Self, from_row: usize, to_row: usize, scale: u8) -> &mut Self {
        return self;
    }

    fn augment(self: &Self)

    pub fn invert(self: &mut Self) -> Matrix {
        return self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero() {
        let zero = Matrix::zero(10, 5);
        assert_eq!(zero.mat.len(), 10);
        assert_eq!(zero.rows, 10);
        assert_eq!(zero.cols, 5);
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

    #[test]
    fn mul_simple() {
        let a = Matrix::try_from(&[&[1u8, 2u8, 3u8][..], &[4u8, 5u8, 6u8][..]][..]).unwrap();
        let b = Matrix::try_from(&[&[1u8, 2u8][..], &[1u8, 2u8][..], &[1u8, 2u8][..]][..]).unwrap();
        let res = a.mul(&b);
        assert_eq!(res.mat[0][0], 6);
        assert_eq!(res.mat[0][1], 12);
        assert_eq!(res.mat[1][0], 15);
        assert_eq!(res.mat[1][1], 30);
    }

    #[test]
    fn mul_id() {
        let m = Matrix::try_from(&[&[1u8, 2u8][..], &[3u8, 4u8][..], &[5u8, 6u8][..]][..]).unwrap();
        assert_eq!(Matrix::identity(3).mul(&m), m);
        assert_eq!(m.mul(&Matrix::identity(2)), m);
    }
}
