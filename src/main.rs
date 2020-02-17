#![feature(test)]

mod chunker;
mod encoder;
mod encoding;
mod finite_field;
mod matrix;
mod polynomial;

use encoder::LagrangeInterpolationEncoder;
use encoder::RSEncoder;
use encoding::Encoding;
use finite_field::DirectField;
use std::str::FromStr;

// TODO:
//
// 1. Implement fast direct inverses used the extended euclidean algorithm. -- too hard, not worth
// 2. Implement interpolation via Vandermonde matrices. -- Done
// 3. Implement interpolation via Cauchy matrices.
// 4. Transpose the output for encode (i.e. output[i] should be a vector of all the i'th indexes).
// 5. Implement architecture-specific improvements.

fn main() {
    let string = "Test string";
    println!("Bytes: {:?}", string.as_bytes());
    let encoding = Encoding::from_str("rs=6.4").expect("Should parse");
    let encoder = LagrangeInterpolationEncoder {};
    let stream = encoder
        .encode_bytes(encoding, &DirectField::default(), string.as_bytes())
        .expect(&format!(
            "Encoding did not work for byte stream: {}",
            string
        ));
    println!("Length: {:?}", stream.length);
    println!("Encoding: {:?}", stream.encoding);
    println!("Codes: {:?}", stream.codes);
}
