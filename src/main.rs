use std::iter;
use std::result::Result;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
struct Encoding {
    data_chunks: u32,
    code_chunks: u32,
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
        let chunks: Vec<_> = s
            .get(3..)
            .expect("string must start with rs=")
            .split(".")
            .map(|x| FromStr::from_str(x))
            .collect();

        match chunks[..] {
            [Ok(total), Ok(code)] => {
                if code <= total {
                    Ok(Encoding {
                        data_chunks: total - code,
                        code_chunks: code,
                    })
                } else {
                    Err("The number of code chunks must be less than or equal to the total number of chunks.")
                }
            }
            _ => Err("Chunks must be specified in the form m.n where m and n are integers."),
        }
    }
}

// A polynomial over byte values.
struct Polynomial {
    // Term coefficients for powers of x starting at 0 (i.e. coefficients[i] is for term cx^i).
    // The last element must always be non-zero. This allows us to efficiently compute the degree
    // from the length of this list without tracking it separately.
    coefficients: Vec<u8>,
}

impl Polynomial {
    // Returns the "zero" polynomial which is defined as the polynomial with no coefficients and
    // degree -1.
    pub fn zero() -> Self {
        Polynomial {
            coefficients: Vec::new(),
        }
    }

    // Creates a Polynomial from a given vector of coefficients. Has degree d == coefficients.len()
    // - 1.
    pub fn from_coefficients(coefficients: &Vec<u8>) -> Self {
        Polynomial {
            coefficients: coefficients.clone(),
        }
    }

    // Computes a single term Polynomial P such that P(i) == values[i].
    pub fn single_term(values: &Vec<u8>, i: u8) -> Self {
        let xi = i;
        let yi = values[i as usize];

        // Computes the term:
        //        ___
        //       |   | (x - xj)
        //  yi * |   | ---------
        //       |   | (xi - xj)
        //      j /= i
        let mut term = Self::from_coefficients(&vec![yi]);
        for (xj, _yj) in values.iter().enumerate().filter(|(x, _y)| *x as u8 != xi) {
            let xj = xj as u8;
            // Equivalent to the term:
            //
            //   (x - xj)
            //   ---------
            //   (xi - xj)
            let denominator = xi.wrapping_sub(xj);
            let zeroth_term = xj.wrapping_neg().wrapping_div(denominator);
            let first_term = 1u8.wrapping_div(denominator);

            term = Self::mul(
                &term,
                &Self::from_coefficients(&vec![zeroth_term, first_term]),
            );
        }

        return term;
    }

    // Generates a polynomial P, such that P(i) == values[i], for i in 0..values.len().
    pub fn interpolate(values: &Vec<u8>) -> Self {
        return Self::zero();
    }

    // Generates a polynomial from the given values. The values are intepreted as y-values for the
    // polynomial with the x-values being their index within the vector. That is to say, for a
    // vector of n values, we would interpolate using [(0, values[0], ..., (n-1, values[n-1])].
    pub fn interpolate(values: &Vec<u8>) -> Self {
        if values.len() == 0 {
            return Self::zero();
        }
        return (0..values.len() - 1)
            .map(|i| Self::single_term(values, i as u8))
            .fold(Self::zero(), |x, y| x.add(&y));
    }

    // Adds to polynomials together. This involves doing a pointwise sum of coefficients.
    pub fn add(self: &Self, other: &Self) -> Self {
        let shorter;
        let longer;
        if self.coefficients.len() > other.coefficients.len() {
            shorter = &other.coefficients;
            longer = &self.coefficients;
        } else {
            shorter = &self.coefficients;
            longer = &other.coefficients;
        }

        // Using wrapping_add gives us addition modulo 256 (u8::max_value()).
        let new_coefficients: Vec<u8> = shorter
            .into_iter()
            .cloned()
            .chain(iter::repeat(0u8))
            .zip(longer)
            .map(|(x, y)| x.wrapping_add(*y))
            .collect();
        return Self::from_coefficients(&new_coefficients);
    }

    // Returns the degree of the Polynomial which is defined as -1 for the zero Polynomial and the
    // largest exponent (power) of x for any term (e.g. for `5 + x + 2x^3` it is `3`) otherwise,
    // with the constant term having exponent `0`.
    pub fn degree(self: &Self) -> i64 {
        return self.coefficients.len() as i64 - 1;
    }

    // Returns whether this Polynomial is the zero Polynomial.
    pub fn is_zero(self: &Self) -> bool {
        return self.degree() == -1;
    }

    pub fn mul(self: &Self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }

        // Compute the degree of the resulting polynomial as the sum of degrees
        let degree = self.degree() + other.degree();

        let new_coefficients: Vec<u8> = iter::repeat(0u8).take((degree + 1) as usize).collect();
        for (e1, c1) in self.coefficients.iter().enumerate() {
            for (e2, c2) in other.coefficients.iter().enumerate() {
                new_coefficients[e1 + e2] += c1 * c2;
            }
        }

        return Self::from_coefficients(&new_coefficients);
    }

    pub fn evaluate(self: &Self, x: u8) -> u8 {
        let mut result: u8 = 0
        for e, c in self.coefficients.iter().enumerate() {
            //result += 
        } 

        return result;
    }
}

// Reed-Solomon encoded data. Length is used to discard padding bytes added to make the number of
// bytes (u8s) in codes a multiple of the encoding data chunks.
struct RSStream {
    length: u64,
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
}

// Encode a stream of bytes as a list of 8 byte data chunks along with their code chunks.
fn encode_bytes(encoding: Encoding, bytes: &Vec<u8>) -> RSStream {
    if bytes.len() == 0 {
        return RSStream::empty(encoding);
    }

    // Pad out the input vector if it is not a multiple of the encoding's data chunk length so that
    // we have enough data to form a polynomial.
    let padding = encoding.data_chunks - (bytes.len() % (encoding.data_chunks as usize));
    let iterations = (bytes.len() + padding) / encoding.data_chunks;
    let input = bytes.iter().cloned().chain(iter::repeat(0u8).take(padding));

    // Generate our interpolated polynomial where P(i) for i from 0..encoding.data_chunks ==
    // input[i * k] (where k is the iteration of bytes we are encoding).
    let mut output: Vec<Vec<u8>> = Vec::with_capacity(iterations);
    for k in (0..iterations) {
        let p = Polynomial::interpolate(input);
        output[k] = Vec::with_capacity(encoding.data_chunks + encoding.code_chunks);
    }

    return RSStream::empty(encoding);
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_good() {
        let expected = Encoding {
            data_chunks: 5,
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
        let actual: Result<Encoding, _> = FromStr::from_str("rs=9.10");
        assert_eq!(actual.is_err(), true);
    }
}
