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
}

impl RSStream {
    pub fn empty(encoding: Encoding) -> Self {
        RSStream {
            length: 0,
            encoding: encoding,
            codes: Vec::new(),
        }
    }

    // Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
    pub fn encode_bytes(encoding: Encoding, bytes: &[u8]) -> RSStream {
        if bytes.len() == 0 {
            return RSStream::empty(encoding);
        }

        // Pad out the input vector if it is not a multiple of the encoding's data chunk length so that
        // we have enough data to form a polynomial.
        let padding = encoding.data_chunks - (bytes.len() % (encoding.data_chunks as usize)) as u8;
        let iterations = (bytes.len() + padding as usize) / encoding.data_chunks as usize;
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

        return RSStream {
            length: bytes.len(),
            encoding: encoding,
            codes: output,
        };
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
}
