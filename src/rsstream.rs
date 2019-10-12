use crate::encoding::Encoding;
use crate::finite_field::Field256;
use crate::polynomial::Polynomial;
use std::iter;

// Reed-Solomon encoded data. Length is used to discard padding bytes added to make the number of
// bytes (u8s) in codes a multiple of the encoding data chunks.
#[derive(Debug, PartialEq)]
pub struct RSStream {
    pub length: usize,
    pub encoding: Encoding,
    pub codes: Vec<Vec<u8>>,
    // True for [i] if there was an erasure in codes[i].
    pub erasures: Vec<bool>,
}

impl RSStream {
    pub fn empty(encoding: Encoding) -> Self {
        RSStream {
            length: 0,
            encoding: encoding,
            codes: Vec::new(),
            erasures: Vec::new(),
        }
    }
}

pub trait RSEncoder {
    fn encode_bytes<F: Field256>(encoding: Encoding, field: &F, bytes: &[u8]) -> RSStream;
    fn decode_bytes<F: Field256>(stream: &RSStream, field: &F) -> Result<Vec<u8>, String>;
}

// Encoder using lagrangian interpolation to construct Polynomials given a set of points. Slow.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct LagrangeInterpolationEncoder;

impl RSEncoder for LagrangeInterpolationEncoder {
    // Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
    fn encode_bytes<F: Field256>(encoding: Encoding, field: &F, bytes: &[u8]) -> RSStream {
        if bytes.len() == 0 {
            return RSStream::empty(encoding);
        }

        // The number of stripes
        let iterations = bytes.len() / encoding.data_chunks as usize;
        // Pad out the input vector if it is not a multiple of the encoding's data chunk length so
        // that we have enough data to form a polynomial.
        let padding = encoding.data_chunks - (bytes.len() % (encoding.data_chunks as usize)) as u8;
        // let input: Vec<u8> = bytes
        //     .into_iter()
        //     .cloned()
        //     .chain(iter::repeat(0u8).take(padding as usize))
        //     .collect();
        let input = &mut bytes
            .into_iter()
            .cloned()
            .chain(iter::repeat(0u8).take(padding as usize));

        // Generate our interpolated polynomial where P(i) for i from 0..encoding.data_chunks ==
        // input[i * k] (where k is the iteration of bytes we are encoding).
        let mut output: Vec<Vec<u8>> = Vec::with_capacity(iterations);
        for k in 0..iterations {
            // let start = k * encoding.data_chunks as usize;
            // let end = start + encoding.data_chunks as usize;
            // let v = &input[start..end];
            let v: Vec<u8> = input.take(encoding.data_chunks as usize).collect();
            println!("Chunk: {:?}", v);
            let p = Polynomial::interpolate(&v[..], field);
            println!("Interpolated: {:?}", p);
            output.push(Vec::with_capacity(
                (encoding.data_chunks + encoding.code_chunks) as usize,
            ));

            let chunks = encoding.data_chunks + encoding.code_chunks;
            for i in 0..chunks {
                output[k].push(p.evaluate(i, field));
                println!(
                    "k: {:?}, i: {:?}, value: {:?}",
                    k,
                    i,
                    p.evaluate(i as u8, field)
                );
            }
        }

        let total_chunks = encoding.total_chunks();
        return RSStream {
            length: bytes.len(),
            encoding: encoding,
            codes: output,
            erasures: iter::repeat(false).take(total_chunks).collect(),
        };
    }

    fn decode_bytes<F: Field256>(stream: &RSStream, field: &F) -> Result<Vec<u8>, String> {
        let RSStream {
            length,
            encoding,
            codes,
            erasures,
        } = stream;
        if *length == 0 {
            return Ok(Vec::new());
        }
        if erasures.iter().filter(|x| **x).map(|_x| 1).sum::<u8>() > encoding.code_chunks {
            return Err(String::from("Too many erasures to recover"));
        }

        let mut res = Vec::with_capacity(*length);
        if erasures
            .iter()
            .take(encoding.data_chunks as usize)
            .all(|x| !*x)
        {
            for i in 0..*length {
                let row = i / encoding.data_chunks as usize;
                let col = i % encoding.data_chunks as usize;
                res.push(codes[row][col]);
            }
            return Ok(res);
        } else {
            // First, figure out which indices are valid.
            let valid_indices: Vec<_> = erasures
                .iter()
                .enumerate()
                .filter(|(_, y)| !**y)
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
}

// Encoder using Vandermonde matrices to do polynomial interpolation.
// pub struct VandermondeEncoder {
//     matrix: Vec<Vec<u8>>,
// }

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
// impl VandermondeEncoder {
//     // Generates a new Vandermonde matrix to be used with a given encoding and inverts it. The
//     // result is multiplied by the input data points to generate the polynomial coefficients that
//     // would generate those points.
//     pub fn new()encoding: Encoding) {}
// }

// impl RSEncoder for VandermondeEncoder {
//     fn encode_bytes<F: Field256>(encoding: Encoding, field: &F, bytes: &[u8]) -> RSStream {
//     }
//     j
//     fn decode_bytes<F: Field256>(stream: &RSStream, field: &F) -> Result<Vec<u8>, String> {
//     }
// }

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
        assert_eq!(
            LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &[]),
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
            erasures: vec![false, false, false, false, false, false],
        };
        assert_eq!(
            LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes),
            expected
        );
    }

    #[bench]
    fn encode_bytes_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        b.iter(|| LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes[..]));
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
            erasures: vec![false, false, false, false, false, false],
        };
        let res = LagrangeInterpolationEncoder::decode_bytes(&input, &direct);
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
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoded = LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes[..]);
        b.iter(|| LagrangeInterpolationEncoder::decode_bytes(&encoded, &direct));
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
            erasures: vec![false, false, false, false, true, true],
        };
        let res = LagrangeInterpolationEncoder::decode_bytes(&input, &direct);
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
        let mut encoded = LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes[..]);
        encoded.erasures = vec![false, false, false, false, true, true];
        b.iter(|| LagrangeInterpolationEncoder::decode_bytes(&encoded, &direct));
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
            erasures: vec![true, false, true, false, false, false],
        };
        let res = LagrangeInterpolationEncoder::decode_bytes(&input, &direct);
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
        let mut encoded = LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes[..]);
        encoded.erasures = vec![true, false, true, false, false, false];
        b.iter(|| LagrangeInterpolationEncoder::decode_bytes(&encoded, &direct));
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
            erasures: vec![true, true, true, false, false, false],
        };
        let res = LagrangeInterpolationEncoder::decode_bytes(&input, &direct);
        assert_eq!(res.is_err(), true);
    }

    #[bench]
    fn decode_bytes_too_many_erasures_4k(b: &mut Bencher) {
        let direct = ExpLogField::default();
        let kilobyte_4 = 4 << 10;
        let bytes: Vec<u8> = (0..kilobyte_4).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let mut encoded = LagrangeInterpolationEncoder::encode_bytes(encoding, &direct, &bytes[..]);
        encoded.erasures = vec![true, true, true, false, false, false];
        b.iter(|| LagrangeInterpolationEncoder::decode_bytes(&encoded, &direct));
    }
}
