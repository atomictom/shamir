mod polynomial;

use polynomial::Polynomial;
use std::iter;
use std::result::Result;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
struct Encoding {
    data_chunks: u8,
    code_chunks: u8,
}

impl FromStr for Encoding {
    type Err = &'static str;

    // Read an encoding of the form rs=n.m where n is the number of total chunks and m is the
    // number of code chunks. Put differently, m is the number of chunks we can loose and still
    // reconstruct all chunks.
    fn from_str(s: &str) -> Result<Encoding, Self::Err> {
        if !s.starts_with("rs=") {
            return Err("Encodings must start with \"rs=\"");
        }
        let chunks: Vec<Result<u8, _>> = s
            .get(3..)
            .expect("string must start with rs=")
            .split(".")
            .map(|x| FromStr::from_str(x))
            .collect();

        match chunks[..] {
            [Ok(data), Ok(code)] => {
                if data.checked_add(code).is_some() {
                    Ok(Encoding {
                        data_chunks: data,
                        code_chunks: code,
                    })
                } else {
                    Err("Total number of chunks must be less than 256.")
                }
            }
            _ => Err("Chunks must be specified in the form m.n where m and n are integers."),
        }
    }
}

// Reed-Solomon encoded data. Length is used to discard padding bytes added to make the number of
// bytes (u8s) in codes a multiple of the encoding data chunks.
struct RSStream {
    length: usize,
    encoding: Encoding,
    codes: Vec<Vec<u8>>,
}

impl RSStream {
    fn empty(encoding: Encoding) -> Self {
        RSStream {
            length: 0,
            encoding: encoding,
            codes: Vec::new(),
        }
    }

    fn encoded(length: usize, encoding: Encoding, codes: Vec<Vec<u8>>) -> Self {
        RSStream {
            length: length,
            encoding: encoding,
            codes: codes,
        }
    }
}

// Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
fn encode_bytes(encoding: Encoding, bytes: &[u8]) -> RSStream {
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

    return RSStream::encoded(bytes.len(), encoding, output);
}

fn main() {
    let string = "Test string";
    println!("Bytes: {:?}", string.as_bytes());
    let encoding = Encoding::from_str("rs=6.4").expect("Should parse");
    let stream = encode_bytes(encoding, string.as_bytes());
    println!("Length: {:?}", stream.length);
    println!("Encoding: {:?}", stream.encoding);
    println!("Codes: {:?}", stream.codes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_good() {
        let expected = Encoding {
            data_chunks: 9,
            code_chunks: 4,
        };
        let actual: Result<Encoding, _> = FromStr::from_str("rs=9.4");
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn from_str_no_code_chunks() {
        let expected = Encoding {
            data_chunks: 5,
            code_chunks: 0,
        };
        let actual: Result<Encoding, _> = FromStr::from_str("rs=5.0");
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn from_str_invalid_format() {
        let actual: Result<Encoding, _> = FromStr::from_str("9.4");
        assert_eq!(actual.is_err(), true);
    }

    #[test]
    fn from_str_invalid_encoding() {
        let actual: Result<Encoding, _> = FromStr::from_str("rs=128.128");
        assert_eq!(actual.is_err(), true);
    }
}
