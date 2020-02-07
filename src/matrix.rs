use crate::finite_field::{DirectField, Field256, Ring};
use std::convert::TryFrom;
use std::fmt::Display;
use std::iter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    pub mat: Vec<Vec<u8>>,
}

impl Display for Matrix {
    fn fmt(self: &Self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        for i in 0..self.rows {
            formatter.write_str("\n")?;
            for j in 0..self.cols {
                if j > 0 {
                    formatter.write_str(" ")?;
                }
                formatter.write_str(format!("{}", self.mat[i][j]).as_str())?;
            }
        }
        return Ok(());
    }
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

impl TryFrom<Vec<Vec<u8>>> for Matrix {
    type Error = &'static str;

    fn try_from(elems: Vec<Vec<u8>>) -> Result<Self, Self::Error> {
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

    pub fn mul<F: Field256>(self: &Self, other: &Self, field: &F) -> Matrix {
        assert!(self.cols == other.rows);
        let mut res = Matrix::zero(self.rows, other.cols);
        // Set each element of the matrix
        for i in 0..res.rows {
            for j in 0..res.cols {
                // Calculate a matrix element
                for k in 0..self.cols {
                    res.mat[i][j] =
                        F::add(res.mat[i][j], field.mul(self.mat[i][k], other.mat[k][j]));
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

    fn scale_row<F: Field256>(self: &mut Self, row: usize, scale: u8, field: &F) -> &mut Self {
        for i in 0..self.cols {
            self.mat[row][i] = field.mul(self.mat[row][i], scale);
        }
        return self;
    }

    fn add_scaled_row<F: Field256>(
        self: &mut Self,
        from_row: usize,
        to_row: usize,
        scale: u8,
        field: &F,
    ) -> &mut Self {
        for i in 0..self.cols {
            self.mat[to_row][i] =
                F::add(self.mat[to_row][i], field.mul(self.mat[from_row][i], scale));
        }
        return self;
    }

    fn augment_with_identity(self: &mut Self) -> &mut Self {
        for i in 0..self.rows {
            for j in 0..self.cols {
                self.mat[i].push(if i == j { 1 } else { 0 });
            }
        }
        self.cols *= 2;
        return self;
    }

    pub fn transpose(self: &Self) -> Self {
        let mut mat = Vec::with_capacity(self.cols);
        for i in 0..self.cols {
            mat.push(Vec::with_capacity(self.rows));
            for j in 0..self.rows {
                mat[i].push(self.mat[j][i]);
            }
        }
        return Matrix {
            rows: self.cols,
            cols: self.rows,
            mat: mat,
        };
    }

    pub fn invert<F: Field256>(self: &Self, field: &F) -> Result<Self, &'static str> {
        let mut res = self.clone();
        res.augment_with_identity();

        // Upper triangular reduction
        for i in 0..self.rows {
            // Swap rows, if necessary.
            for j in i..self.rows {
                if self.mat[j][i] != 0 {
                    res.swap_row(i, j);
                    break;
                }
            }
            // If swapping rows did not find a row without a 0 in the row and column we're
            // operating on then the matrix must not be invertable.
            if res.mat[i][i] == 0 {
                return Err("The matrix is singular and cannot be inverted.");
            }
            if res.mat[i][i] != 1 {
                res.scale_row(i, field.inv(res.mat[i][i]), field);
            }
            for j in (i + 1)..self.rows {
                if res.mat[j][i] != 0 {
                    res.add_scaled_row(i, j, res.mat[j][i], field);
                }
            }
        }

        // Lower triangular reduction
        for i in (0..self.rows).rev() {
            for j in 0..i {
                res.add_scaled_row(i, j, res.mat[j][i], field);
            }
        }

        let mut ret_mat = Vec::with_capacity(res.rows);
        for i in 0..self.rows {
            ret_mat.push(Vec::with_capacity(res.cols));
            for j in 0..self.cols {
                ret_mat[i].push(res.mat[i][j + self.cols]);
            }
        }
        return Ok(Matrix {
            rows: self.rows,
            cols: self.cols,
            mat: ret_mat,
        });
    }
}

pub fn VandermondeMatrix<F: Field256>(
    start: usize,
    rows: usize,
    cols: usize,
    field: &F,
) -> Result<Matrix, &'static str> {
    let mut matrix = Vec::with_capacity(rows);
    for i in start..(start + rows) {
        let mut row = Vec::with_capacity(cols);
        for j in 0..cols {
            row.push(field.exp(i as u8, j as u8));
        }
        matrix.push(row);
    }
    // Creating this should not ever fail.
    return Matrix::try_from(matrix);
}

pub fn PartialVandermondeMatrix<F: Field256, I: Iterator<Item = bool>>(
    rows: I,
    cols: usize,
    field: &F,
) -> Result<Matrix, &'static str> {
    let mut matrix = Vec::with_capacity(cols);
    for (i, _) in rows.enumerate().filter(|(_, x)| *x) {
        let mut row = Vec::with_capacity(cols);
        for j in 0..cols {
            row.push(field.exp(i as u8, j as u8));
        }
        matrix.push(row);
    }
    // Creating this should not ever fail.
    return Matrix::try_from(matrix);
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
    fn invert_identity_is_identity() {
        let direct = DirectField::default();
        let id = Matrix::identity(5);
        let inv = id.invert(&direct).unwrap();
        assert_eq!(id, inv);
    }

    #[test]
    fn mat_mul_by_inv_is_identity() {
        let direct = DirectField::default();

        let a = Matrix::try_from(
            &[
                &[1u8, 2u8, 3u8][..],
                &[4u8, 5u8, 6u8][..],
                &[5u8, 6u8, 7u8][..],
            ][..],
        )
        .unwrap();
        let a_inv = a.invert(&direct).unwrap();
        assert_eq!(a.mul(&a_inv, &direct), Matrix::identity(3));
    }

    #[test]
    fn mul_simple() {
        // This gives us "normal" multiplication, but inv/div is broken. That's okay for this test
        // and it makes it easier to verify the multiplication works right.
        let ring = Ring::default();
        let a = Matrix::try_from(&[&[1u8, 2u8, 3u8][..], &[4u8, 5u8, 6u8][..]][..]).unwrap();
        let b = Matrix::try_from(&[&[1u8, 2u8][..], &[1u8, 2u8][..], &[1u8, 2u8][..]][..]).unwrap();
        let res = a.mul(&b, &ring);
        assert_eq!(res.mat[0][0], 6);
        assert_eq!(res.mat[0][1], 12);
        assert_eq!(res.mat[1][0], 15);
        assert_eq!(res.mat[1][1], 30);
    }

    #[test]
    fn mul_id() {
        let direct = DirectField::default();
        let m = Matrix::try_from(&[&[1u8, 2u8][..], &[3u8, 4u8][..], &[5u8, 6u8][..]][..]).unwrap();
        assert_eq!(Matrix::identity(3).mul(&m, &direct), m);
        assert_eq!(m.mul(&Matrix::identity(2), &direct), m);
    }
}
