use crate::chunker::ChunkerExt;
use crate::encoding::Encoding;
use crate::finite_field::Field256;
use crate::matrix::Matrix;
use crate::matrix::PartialVandermondeMatrix;
use crate::matrix::VandermondeMatrix;
use crate::polynomial::Polynomial;
use std::convert::TryFrom;
use std::iter;

// Reed-Solomon encoded data.
#[derive(Debug, PartialEq)]
pub struct RSStream {
    // Length is used to discard padding bytes added to make the number of
    // bytes (u8s) in codes a multiple of the encoding data chunks.
    pub length: usize,
    // How the reed-solomon data has been encoded.
    pub encoding: Encoding,
    // codes has list per chunk with the values being those within the chunk.
    pub codes: Vec<Vec<u8>>,
    // True for [i] if there was NOT an erasure in codes[*][i]. Can be empty if there is no erasure
    // data.
    pub valid: Vec<bool>,
}

impl RSStream {
    pub fn empty(encoding: Encoding) -> Self {
        RSStream {
            length: 0,
            encoding: encoding,
            codes: Vec::new(),
            valid: Vec::new(),
        }
    }
}

pub trait RSEncoder {
    fn encode_bytes<F: Field256>(
        &self,
        encoding: Encoding,
        field: &F,
        bytes: &[u8],
    ) -> Result<RSStream, String>;
    fn decode_bytes<F: Field256>(&self, stream: &RSStream, field: &F) -> Result<Vec<u8>, String>;
}

// Encoder using lagrangian interpolation to construct Polynomials given a set of points. Slow.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct LagrangeInterpolationEncoder;

impl RSEncoder for LagrangeInterpolationEncoder {
    // Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
    fn encode_bytes<F: Field256>(
        &self,
        encoding: Encoding,
        field: &F,
        bytes: &[u8],
    ) -> Result<RSStream, String> {
        if bytes.len() == 0 {
            return Ok(RSStream::empty(encoding));
        }

        // The number of chunks.
        let iterations = bytes.len() / encoding.data_chunks as usize;

        // Generate our interpolated polynomial where P(i) for i from 0..encoding.data_chunks ==
        // input[i * k] (where k is the iteration of bytes we are encoding).
        let mut output: Vec<Vec<u8>> = Vec::with_capacity(iterations);
        for (i, chunk) in bytes
            .iter()
            .cloned()
            .chunked_with_default(encoding.data_chunks as usize, 0)
            .enumerate()
        {
            let p = Polynomial::interpolate(&chunk[..], field);
            output.push(Vec::with_capacity(encoding.total_chunks() as usize));

            for b in 0..encoding.total_chunks() {
                // Only evaluate the polynomial for code chunks.
                if b < encoding.data_chunks {
                    output[i].push(chunk[b as usize]);
                } else {
                    output[i].push(p.evaluate(b, field));
                }
            }
        }

        return Ok(RSStream {
            length: bytes.len(),
            encoding: encoding,
            codes: output,
            valid: Vec::new(),
        });
    }

    fn decode_bytes<F: Field256>(&self, stream: &RSStream, field: &F) -> Result<Vec<u8>, String> {
        let RSStream {
            length,
            encoding,
            codes,
            valid,
        } = stream;
        if *length == 0 {
            return Ok(Vec::new());
        }
        if valid.iter().cloned().filter(|x| *x).count() < encoding.data_chunks as usize {
            return Err(String::from("Too many erasures to recover"));
        }

        let mut res = Vec::with_capacity(*length);

        // Fast path with no data erasures
        if valid
            .iter()
            .cloned()
            .take(encoding.data_chunks as usize)
            .all(|x| x)
        {
            for i in 0..*length {
                let row = i / encoding.data_chunks as usize;
                let col = i % encoding.data_chunks as usize;
                res.push(codes[row][col]);
            }
            return Ok(res);
        }

        // Slow path with erasures

        // First, figure out which indices are valid.
        let valid_indices: Vec<_> = valid
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, y)| *y)
            .map(|(x, _)| x)
            .take(encoding.data_chunks as usize)
            .collect();

        // Now, for each input row, interpolate the polynomial and then generate our data
        // points.
        let rows = length / encoding.data_chunks as usize;
        for row in 0..rows {
            let row = row as usize;
            let points: Vec<_> = valid_indices
                .iter()
                .map(|col| (*col as u8, codes[row][*col as usize]))
                .collect();
            let p = Polynomial::interpolate_points(&points[..], field);
            for col in 0..encoding.data_chunks {
                let i = row * encoding.data_chunks as usize + col as usize;
                res.insert(i, p.evaluate(col as u8, field));
            }
        }

        return Ok(res);
    }
}

// rs=3.2
//
// [1 0 0]         [A]
// [0 1 0]   [A]   [B]
// [0 0 1] * [B] = [C]
// [? ? ?]   [C]   [X]
// [? ? ?]         [Z]
//
//         [1 0 0]^-1     [A]
//  [A]   ([0 1 0])      ([B])
//  [B] =  [0 0 1]     *  [C]
//  [C]    [? ? ?]        [X]
//        ([? ? ?])      ([Z])
//
// 1 0 0     C0
// 1 1 1  *  C1
// 1 2 4     C2
// 1 3 9
//
// i0^2 + j0^1 + k0^0 = a
// i1^2 + j1^1 + k1^0 = b
// i2^2 + j2^1 + k2^0 = c
//
//   5*3      3*1   5*1
// [1 0  0]         [a]
// [1 1  1]   [i]   [b]
// [1 2  4] * [j] = [c]
// [1 3  9]   [k]   [x]
// [1 4 16]         [y]
//
// 3*1   5*1   5*3
//       [a]   [1 0  0]^-1
// [i]   [b]   [1 1  1]
// [j] = [c] * [1 2  4]
// [k]   [x]   [1 3  9]
//       [y]   [1 4 16]
//
//
// CX = P
// C = D * X^-1

// Encoder using Vandermonde matrices to do polynomial interpolation.
#[derive(Debug, Clone)]
pub struct VandermondeEncoder {
    // The inverted vandermonde matrix used to compute polynomial coefficients given a set of data
// points.
// inverted: Matrix,
// A Vandermonde matrix for generating code or data points with the polynomial coefficients.
// vandermonde: Matrix,
}

// impl VandermondeEncoder {
// Generates a new Vandermonde matrix to be used with a given encoding and inverts it. The
// result will be multiplied by the input data points to generate the polynomial coefficients
// that would generate those points.
// pub fn new<F: Field256>(encoding: &Encoding, field: &F) -> Result<VandermondeEncoder, String> {
//     let inverted = VandermondeMatrix(
//         0,
//         encoding.data_chunks as usize,
//         encoding.data_chunks as usize,
//         field,
//     )?
//     .invert(field)?;
//
//     let vandermonde = VandermondeMatrix(
//         encoding.data_chunks as usize,
//         encoding.code_chunks as usize,
//         encoding.data_chunks as usize,
//         field,
//     )?;
//
//     return Ok(VandermondeEncoder {
//         inverted: inverted,
//         vandermonde: vandermonde,
//     });
// }
// }

impl RSEncoder for VandermondeEncoder {
    fn encode_bytes<F: Field256>(
        &self,
        encoding: Encoding,
        field: &F,
        bytes: &[u8],
    ) -> Result<RSStream, String> {
        if bytes.len() == 0 {
            return Ok(RSStream::empty(encoding));
        }

        let inverted = VandermondeMatrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .invert(field)?;

        let vandermonde = VandermondeMatrix(
            encoding.data_chunks as usize,
            encoding.code_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?;

        // The number of chunks.
        let iterations = bytes.len() / encoding.data_chunks as usize;

        // Generate data one "chunk" at a time (i.e. the data symbols and the code symbols).
        let mut output: Vec<Vec<u8>> = Vec::with_capacity(iterations);
        for (i, chunk) in bytes
            .iter()
            .cloned()
            .chunked_with_default(encoding.data_chunks as usize, 0)
            .enumerate()
        {
            let data = Matrix::try_from(&[&chunk[..]][..])?;
            let coefficients = inverted.mul(&data.transpose(), field);
            let codes = vandermonde.mul(&coefficients, field);

            output.push(Vec::with_capacity(encoding.total_chunks() as usize));
            for b in 0..encoding.data_chunks {
                output[i].push(chunk[b as usize]);
            }
            for b in 0..encoding.code_chunks {
                output[i].push(codes.mat[b as usize][0]);
            }
        }

        return Ok(RSStream {
            length: bytes.len(),
            encoding: encoding,
            codes: output,
            valid: Vec::new(),
        });
    }

    fn decode_bytes<F: Field256>(&self, stream: &RSStream, field: &F) -> Result<Vec<u8>, String> {
        let RSStream {
            length,
            encoding,
            codes,
            valid,
        } = stream;
        if *length == 0 {
            return Ok(Vec::new());
        }
        let valid_indices: Vec<usize> = valid
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, valid)| *valid)
            .map(|(i, _)| i)
            .take(encoding.data_chunks as usize)
            .collect();
        println!("Valid: {:?}", valid_indices);

        if valid_indices.len() < encoding.data_chunks as usize {
            return Err(String::from("Too many erasures to recover"));
        }

        let mut res = Vec::with_capacity(*length);

        // Fast path with no erasures
        if valid
            .iter()
            .cloned()
            .take(encoding.data_chunks as usize)
            .all(|x| x)
        {
            for i in 0..*length {
                let row = i / encoding.data_chunks as usize;
                let col = i % encoding.data_chunks as usize;
                res.push(codes[row][col]);
            }
            return Ok(res);
        }

        // Slow path with erasures.

        // Generate the inverted vandermonde matrix for the valid indices to generate polynomial
        // coefficients.
        let inverted =
            PartialVandermondeMatrix(valid.iter().cloned(), encoding.data_chunks as usize, field)?
                .invert(field)?;

        // Generate the data vandermonde matrix to be used with the coefficients to generate the
        // original data.
        let vandermonde = VandermondeMatrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?;

        // Loop through each chunk and decode it.
        let mut chunk: Vec<u8> = iter::repeat(0)
            .take(encoding.data_chunks as usize)
            .collect();
        let rows = length / encoding.data_chunks as usize;
        for i in 0..rows {
            for (e, j) in valid_indices.iter().cloned().enumerate() {
                chunk[e] = codes[i as usize][j as usize];
            }
            // Some combo of data and/or codes.
            let values = Matrix::try_from(&[&chunk[..]][..])?;
            let coefficients = inverted.mul(&values.transpose(), field);
            let original_data = vandermonde.mul(&coefficients, field);
            for j in 0..encoding.data_chunks {
                res.push(original_data.mat[j as usize][0]);
            }
        }

        return Ok(res);
    }
}

#[cfg(test)]
mod tests {
    extern crate rand;
    extern crate test;
    use super::*;
    // TODO: Consider using Criterion
    use crate::finite_field::{DirectField, ExpLogField};
    use std::str::FromStr;
    use test::Bencher;

    #[test]
    fn encode_bytes_empty() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=9.4").unwrap();
        let expected = RSStream::empty(encoding.clone());
        let encoder = LagrangeInterpolationEncoder {};
        assert_eq!(
            encoder.encode_bytes(encoding, &direct, &[]).unwrap(),
            expected
        );
    }

    #[test]
    fn encode_bytes_small() {
        let direct = DirectField::default();
        let bytes = "DEADBEEF".as_bytes();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let expected = RSStream {
            length: 8,
            encoding: encoding.clone(),
            codes: vec![
                vec![0x44, 0x45, 0x41, 0x44, 0x02, 0x1B],
                vec![0x42, 0x45, 0x45, 0x46, 0x38, 0x27],
            ],
            valid: vec![],
        };
        let encoder = LagrangeInterpolationEncoder {};
        assert_eq!(
            encoder.encode_bytes(encoding, &direct, &bytes).unwrap(),
            expected
        );
    }

    #[test]
    fn encode_decode_vandermonde() {
        let direct = DirectField::default();
        let bytes = "DEADBEEF".as_bytes();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        // let encoder = VandermondeEncoder::new(&encoding, &direct)
        // .expect("Could not construct VandermondeEncoder.");
        let encoder = VandermondeEncoder {};
        let encoded = encoder.encode_bytes(encoding, &direct, &bytes).unwrap();
        let encoded = RSStream {
            valid: vec![true, false, true, true, true, false],
            ..encoded
        };
        let decoder = VandermondeEncoder {};
        let decoded = decoder.decode_bytes(&encoded, &direct);
        assert_eq!(decoded.unwrap(), bytes);
    }

    #[bench]
    fn encode_bytes_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = LagrangeInterpolationEncoder {};
        b.iter(|| encoder.encode_bytes(encoding, &direct, &bytes[..]));
    }

    #[test]
    fn decode_bytes_no_erasures() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let input = RSStream {
            length: 8,
            encoding: encoding.clone(),
            codes: vec![
                vec![0x44, 0x45, 0x41, 0x44, 0x02, 0x1B],
                vec![0x42, 0x45, 0x45, 0x46, 0x38, 0x27],
            ],
            valid: vec![true, true, true, true, true, true],
        };
        let encoder = LagrangeInterpolationEncoder {};
        let res = encoder.decode_bytes(&input, &direct);
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[bench]
    fn decode_bytes_no_erasures_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoder = LagrangeInterpolationEncoder {};
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[test]
    fn decode_bytes_code_erasure() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let input = RSStream {
            length: 8,
            encoding: encoding.clone(),
            codes: vec![
                vec![0x44, 0x45, 0x41, 0x44, 0x00, 0x00],
                vec![0x42, 0x45, 0x45, 0x46, 0x00, 0x00],
            ],
            valid: vec![true, true, true, true, false, false],
        };
        let encoder = LagrangeInterpolationEncoder {};
        let res = encoder.decode_bytes(&input, &direct);
        assert!(res.is_ok());
        assert_eq!(
            res.expect("Got: "),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[bench]
    fn decode_bytes_code_erasures_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = LagrangeInterpolationEncoder {};
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![true, true, true, true, false, false];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[test]
    fn decode_bytes_data_erasure() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let input = RSStream {
            length: 8,
            encoding: encoding.clone(),
            codes: vec![
                vec![0x00, 0x45, 0x00, 0x44, 0x02, 0x1B],
                vec![0x00, 0x45, 0x00, 0x46, 0x38, 0x27],
            ],
            valid: vec![false, true, false, true, true, true],
        };
        let encoder = LagrangeInterpolationEncoder {};
        let res = encoder.decode_bytes(&input, &direct);
        assert_eq!(
            res.expect("Got: "),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[bench]
    fn decode_bytes_data_erasures_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = LagrangeInterpolationEncoder {};
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![false, true, false, true, true, true];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[test]
    fn decode_bytes_too_many_erasures() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let input = RSStream {
            length: 8,
            encoding: encoding.clone(),
            codes: vec![
                vec![0x00, 0x00, 0x00, 0x44, 0x02, 0x1B],
                vec![0x00, 0x00, 0x00, 0x46, 0x38, 0x27],
            ],
            valid: vec![false, false, false, true, true, true],
        };
        let encoder = LagrangeInterpolationEncoder {};
        let res = encoder.decode_bytes(&input, &direct);
        assert_eq!(res.is_err(), true);
    }

    #[bench]
    fn decode_bytes_too_many_erasures_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = LagrangeInterpolationEncoder {};
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![false, false, false, true, true, true];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }
}
