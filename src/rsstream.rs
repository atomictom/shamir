use crate::encoding::Encoding;
use crate::polynomial::Polynomial;
use std::iter;
use std::str::FromStr;

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

    // Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
    pub fn encode_bytes(encoding: Encoding, bytes: &[u8]) -> RSStream {
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
            let p = Polynomial::interpolate(&v[..]);
            println!("Interpolated: {:?}", p);
            output.push(Vec::with_capacity(
                (encoding.data_chunks + encoding.code_chunks) as usize,
            ));

            let chunks = encoding.data_chunks + encoding.code_chunks;
            for i in 0..chunks {
                output[k].push(p.evaluate(i));
                println!("k: {:?}, i: {:?}, value: {:?}", k, i, p.evaluate(i as u8));
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

    pub fn decode_bytes(self: Self) -> Result<Vec<u8>, &'static str> {
        let RSStream {
            length,
            encoding,
            codes,
            erasures,
        } = self;
        if length == 0 {
            return Ok(Vec::new());
        }
        if erasures.iter().filter(|x| **x).map(|_x| 1).sum::<u8>() > encoding.code_chunks {
            return Err("Too many erasures to recover");
        }

        if erasures
            .iter()
            .take(encoding.data_chunks as usize)
            .all(|x| !*x)
        {
            let mut res = Vec::with_capacity(length);
            for i in 0..length {
                let row = i / encoding.data_chunks as usize;
                let col = i % encoding.data_chunks as usize;
                res.insert(i, codes[row][col]);
            }
            return Ok(res);
        } else {
            let mut res = Vec::with_capacity(length);
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
                let p = Polynomial::interpolate_points(&points[..]);
                for col in 0..encoding.data_chunks {
                    let i = row * encoding.data_chunks as usize + col as usize;
                    res.insert(i, p.evaluate(col as u8));
                }
            }

            return Ok(res);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_bytes_empty() {
        let encoding: Encoding = FromStr::from_str("rs=9.4").unwrap();
        let expected = RSStream::empty(encoding.clone());
        assert_eq!(RSStream::encode_bytes(encoding, &[]), expected);
    }

    #[test]
    fn encode_bytes_small() {
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
        assert_eq!(RSStream::encode_bytes(encoding, &bytes), expected);
    }

    #[test]
    fn decode_bytes_no_erasures() {
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
        let res = input.decode_bytes();
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[test]
    fn decode_bytes_code_erasure() {
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
        let res = input.decode_bytes();
        assert!(res.is_ok());
        assert_eq!(
            res.expect("Got: "),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }

    #[test]
    fn decode_bytes_data_erasure() {
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
        let res = input.decode_bytes();
        assert_eq!(
            res.expect("Got: "),
            vec![0x44, 0x45, 0x41, 0x44, 0x42, 0x45, 0x45, 0x46]
        );
    }
}
