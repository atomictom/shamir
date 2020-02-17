use crate::chunker::ChunkerExt;
use crate::encoding::Encoding;
use crate::finite_field::Field256;
use crate::matrix::Matrix;
use crate::matrix::{
    cauchy_matrix, partial_cauchy_matrix, partial_vandermonde_matrix, vandermonde_matrix,
};
use crate::polynomial::Polynomial;
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

fn encode_bytes_matrix<F: Field256>(
    encoding: Encoding,
    generator: &Matrix,
    field: &F,
    bytes: &[u8],
) -> Result<RSStream, String> {
    // The number of chunks.
    let iterations = bytes.len() / encoding.data_chunks as usize;

    // Generate data one "chunk" at a time (i.e. the data symbols and the code symbols).
    let mut output: Vec<Vec<u8>> = Vec::with_capacity(iterations);
    let mut buffer: Vec<u8> = iter::repeat(0)
        .take(encoding.code_chunks as usize)
        .collect();
    for (i, chunk) in bytes
        .iter()
        .cloned()
        .chunked_with_default(encoding.data_chunks as usize, 0)
        .enumerate()
    {
        generator.mul_vec(&chunk, &mut buffer, field);

        output.push(Vec::with_capacity(encoding.total_chunks() as usize));
        output[i].extend(chunk.iter().take(encoding.data_chunks as usize));
        output[i].extend(buffer.iter().take(encoding.code_chunks as usize));
    }

    return Ok(RSStream {
        length: bytes.len(),
        encoding: encoding,
        codes: output,
        valid: Vec::new(),
    });
}

fn decode_bytes_matrix<F: Field256>(
    stream: &RSStream,
    generator: &Matrix,
    valid_indices: &[usize],
    field: &F,
) -> Result<Vec<u8>, String> {
    let RSStream {
        length,
        encoding,
        codes,
        valid: _,
    } = stream;
    let mut res = Vec::with_capacity(*length);

    // Slow path with erasures.
    let mut chunk: Vec<u8> = iter::repeat(0)
        .take(encoding.data_chunks as usize)
        .collect();

    let mut buffer: Vec<u8> = iter::repeat(0)
        .take(encoding.data_chunks as usize)
        .collect();

    // Loop through each chunk and decode it.
    let rows = length / encoding.data_chunks as usize;
    for i in 0..rows {
        for (e, j) in valid_indices.iter().cloned().enumerate() {
            chunk[e] = codes[i as usize][j as usize];
        }
        generator.mul_vec(&chunk, &mut buffer, field);
        res.extend(buffer.iter().take(encoding.data_chunks as usize));
    }

    return Ok(res);
}
// Encoder using Vandermonde matrices to do polynomial interpolation.
#[derive(Debug, Clone, Default)]
pub struct VandermondeEncoder {}

// Encoder using Cauchy matrices to do polynomial interpolation.
#[derive(Debug, Clone, Default)]
pub struct CauchyEncoder {}

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

        let inverted = vandermonde_matrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .invert(field)?;

        let generator = vandermonde_matrix(
            encoding.data_chunks as usize,
            encoding.code_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .mul(&inverted, field);
        return encode_bytes_matrix(encoding, &generator, field, bytes);
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

        if valid_indices.len() < encoding.data_chunks as usize {
            return Err(String::from("Too many erasures to recover"));
        }

        // // Fast path with no erasures
        // if valid
        //     .iter()
        //     .cloned()
        //     .take(encoding.data_chunks as usize)
        //     .all(|x| x)
        // {
        //     let mut res = Vec::with_capacity(*length);
        //     for i in 0..*length {
        //         let row = i / encoding.data_chunks as usize;
        //         let col = i % encoding.data_chunks as usize;
        //         res.push(codes[row][col]);
        //     }
        //     return Ok(res);
        // }

        // Generate the inverted vandermonde matrix for the valid indices to generate polynomial
        // coefficients.
        let inverted = partial_vandermonde_matrix(
            valid.iter().cloned(),
            encoding.data_chunks as usize,
            field,
        )?
        .invert(field)?;

        // Generate the data vandermonde matrix to be used with the coefficients to generate the
        // original data.
        let generator = vandermonde_matrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .mul(&inverted, field);
        return decode_bytes_matrix(stream, &generator, &valid_indices[..], field);
    }
}

impl RSEncoder for CauchyEncoder {
    fn encode_bytes<F: Field256>(
        &self,
        encoding: Encoding,
        field: &F,
        bytes: &[u8],
    ) -> Result<RSStream, String> {
        if bytes.len() == 0 {
            return Ok(RSStream::empty(encoding));
        }

        let inverted = cauchy_matrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .invert(field)?;

        let generator = cauchy_matrix(
            encoding.data_chunks as usize,
            encoding.code_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .mul(&inverted, field);
        return encode_bytes_matrix(encoding, &generator, field, bytes);
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

        if valid_indices.len() < encoding.data_chunks as usize {
            return Err(String::from("Too many erasures to recover"));
        }

        // Generate the inverted cauchy matrix for the valid indices to generate polynomial
        // coefficients.
        let inverted =
            partial_cauchy_matrix(valid.iter().cloned(), encoding.data_chunks as usize, field)?
                .invert(field)?;

        // Generate the data cauchy matrix to be used with the coefficients to generate the
        // original data.
        let generator = cauchy_matrix(
            0,
            encoding.data_chunks as usize,
            encoding.data_chunks as usize,
            field,
        )?
        .mul(&inverted, field);
        return decode_bytes_matrix(stream, &generator, &valid_indices[..], field);
    }
}

#[cfg(test)]
mod tests {
    extern crate rand;
    extern crate test;
    use super::*;
    // TODO: Consider using Criterion
    use crate::finite_field::{DirectField, ExpLogField, TableField};
    use std::str::FromStr;
    use test::Bencher;

    fn encode_bytes_empty<E: RSEncoder + Default>() {
        let direct = DirectField::default();
        let encoding: Encoding = FromStr::from_str("rs=9.4").unwrap();
        let expected = RSStream::empty(encoding.clone());
        let encoder = E::default();
        assert_eq!(
            encoder.encode_bytes(encoding, &direct, &[]).unwrap(),
            expected
        );
    }

    #[test]
    fn encode_bytes_empty_lagrange() {
        encode_bytes_empty::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn encode_bytes_empty_vandermonde() {
        encode_bytes_empty::<VandermondeEncoder>();
    }

    fn encode_bytes_small<E: RSEncoder + Default>() {
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
        let encoder = E::default();
        assert_eq!(
            encoder.encode_bytes(encoding, &direct, &bytes).unwrap(),
            expected
        );
    }

    #[test]
    fn encode_bytes_small_lagrange() {
        encode_bytes_small::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn encode_bytes_small_vandermonde() {
        encode_bytes_small::<VandermondeEncoder>();
    }

    fn encode_bytes<E: RSEncoder + Default, F: Field256 + Default>(b: &mut Bencher, size: usize) {
        let direct = F::default();
        let bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = E::default();
        b.iter(|| encoder.encode_bytes(encoding, &direct, &bytes[..]));
    }

    #[bench]
    #[ignore]
    fn encode_bytes_4k_lagrange(b: &mut Bencher) {
        let size = 4 << 10;
        encode_bytes::<LagrangeInterpolationEncoder, ExpLogField>(b, size);
    }

    #[bench]
    fn encode_bytes_4k_vandermonde_explog(b: &mut Bencher) {
        let size = 4 << 10;
        encode_bytes::<VandermondeEncoder, ExpLogField>(b, size);
    }

    #[bench]
    fn encode_bytes_4k_vandermonde_table(b: &mut Bencher) {
        let size = 4 << 10;
        encode_bytes::<VandermondeEncoder, TableField>(b, size);
    }

    #[bench]
    fn encode_bytes_4k_cauchy_explog(b: &mut Bencher) {
        let size = 4 << 10;
        encode_bytes::<CauchyEncoder, ExpLogField>(b, size);
    }

    #[bench]
    fn encode_bytes_4k_cauchy_table(b: &mut Bencher) {
        let size = 4 << 10;
        encode_bytes::<CauchyEncoder, TableField>(b, size);
    }

    #[bench]
    #[ignore]
    fn encode_bytes_1m_vandermonde_explog(b: &mut Bencher) {
        let size = 1 << 20;
        encode_bytes::<VandermondeEncoder, ExpLogField>(b, size);
    }

    #[bench]
    #[ignore]
    fn encode_bytes_1m_vandermonde_table(b: &mut Bencher) {
        let size = 1 << 20;
        encode_bytes::<VandermondeEncoder, TableField>(b, size);
    }

    fn decode_bytes_no_erasures<E: RSEncoder + Default>() {
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
        let encoder = E::default();
        let res = encoder.decode_bytes(&input, &direct);
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[test]
    fn decode_bytes_no_erasures_lagrange() {
        decode_bytes_no_erasures::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn decode_bytes_no_erasures_vandermonde() {
        decode_bytes_no_erasures::<VandermondeEncoder>();
    }

    fn decode_bytes_no_erasures_bench<E: RSEncoder + Default>(b: &mut Bencher, size: usize) {
        let direct = ExpLogField::default();
        let bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let encoder = E::default();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[bench]
    #[ignore]
    fn decode_bytes_no_erasures_4k_lagrange(b: &mut Bencher) {
        decode_bytes_no_erasures_bench::<LagrangeInterpolationEncoder>(b, 4 << 10);
    }

    #[bench]
    fn decode_bytes_no_erasures_4k_vandermonde(b: &mut Bencher) {
        decode_bytes_no_erasures_bench::<VandermondeEncoder>(b, 1 << 10);
    }

    fn decode_bytes_code_erasure<E: RSEncoder + Default>() {
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

    #[test]
    fn decode_bytes_code_erasure_lagrange() {
        decode_bytes_code_erasure::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn decode_bytes_code_erasures_vandermonde() {
        decode_bytes_code_erasure::<VandermondeEncoder>();
    }

    fn decode_bytes_code_erasures_bench<E: RSEncoder + Default>(b: &mut Bencher, size: usize) {
        let direct = ExpLogField::default();
        let bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = LagrangeInterpolationEncoder {};
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![true, true, true, true, false, false];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[bench]
    #[ignore]
    fn decode_bytes_code_erasures_4k_lagrange(b: &mut Bencher) {
        decode_bytes_code_erasures_bench::<LagrangeInterpolationEncoder>(b, 4 << 10);
    }

    #[bench]
    fn decode_bytes_code_erasures_4k_vandermonde(b: &mut Bencher) {
        decode_bytes_code_erasures_bench::<VandermondeEncoder>(b, 4 << 10);
    }

    fn decode_bytes_data_erasure<E: RSEncoder + Default>() {
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
        let encoder = E::default();
        let res = encoder.decode_bytes(&input, &direct);
        assert_eq!(
            res.expect("Got: "),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[test]
    fn decode_bytes_data_erasure_lagrange() {
        decode_bytes_data_erasure::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn decode_bytes_data_erasures_vandermonde() {
        decode_bytes_data_erasure::<VandermondeEncoder>();
    }

    #[test]
    fn decode_bytes_data_erasures_cauchy() {
        decode_bytes_data_erasure::<CauchyEncoder>();
    }

    fn decode_bytes_data_erasures_bench<E: RSEncoder + Default>(b: &mut Bencher, size: usize) {
        let direct = ExpLogField::default();
        let bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = E::default();
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![false, true, false, true, true, true];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[bench]
    #[ignore]
    fn decode_bytes_data_erasures_4k_lagrange(b: &mut Bencher) {
        decode_bytes_data_erasures_bench::<LagrangeInterpolationEncoder>(b, 4 << 10);
    }

    #[bench]
    fn decode_bytes_data_erasures_4k_vandermonde(b: &mut Bencher) {
        decode_bytes_data_erasures_bench::<VandermondeEncoder>(b, 4 << 10);
    }

    #[bench]
    fn decode_bytes_data_erasures_4k_cauchy(b: &mut Bencher) {
        decode_bytes_data_erasures_bench::<CauchyEncoder>(b, 4 << 10);
    }

    fn decode_bytes_too_many_erasures<E: RSEncoder + Default>() {
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
        let encoder = E::default();
        let res = encoder.decode_bytes(&input, &direct);
        assert_eq!(res.is_err(), true);
    }

    #[test]
    fn decode_bytes_too_many_erasures_lagrange() {
        decode_bytes_too_many_erasures::<LagrangeInterpolationEncoder>();
    }

    #[test]
    fn decode_bytes_too_many_erasures_vandermonde() {
        decode_bytes_too_many_erasures::<VandermondeEncoder>();
    }

    fn decode_bytes_too_many_erasures_bench<E: RSEncoder + Default>(b: &mut Bencher, size: usize) {
        let direct = ExpLogField::default();
        let bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();
        let encoding: Encoding = FromStr::from_str("rs=4.2").unwrap();
        let encoder = E::default();
        let mut encoded = encoder.encode_bytes(encoding, &direct, &bytes[..]).unwrap();
        encoded.valid = vec![false, false, false, true, true, true];
        b.iter(|| encoder.decode_bytes(&encoded, &direct));
    }

    #[bench]
    #[ignore]
    fn decode_bytes_too_many_erasures_4k_lagrange(b: &mut Bencher) {
        decode_bytes_too_many_erasures_bench::<LagrangeInterpolationEncoder>(b, 4 << 10);
    }

    #[bench]
    fn decode_bytes_too_many_erasures_4k_vandermonde(b: &mut Bencher) {
        decode_bytes_too_many_erasures_bench::<VandermondeEncoder>(b, 4 << 10);
    }

}
