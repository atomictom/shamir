#![feature(test)]

mod chunker;
mod encoder;
mod encoding;
mod finite_field;
mod matrix;
mod polynomial;
mod shamir;
mod words;

use crate::shamir::*;
use encoder::RSEncoder;
use encoder::VandermondeEncoder;
use encoding::Encoding;
use finite_field::ExpLogField;
use std::str::FromStr;

// TODO:
//
// 1. Implement fast direct inverses used the extended euclidean algorithm. -- too hard, not worth
// 2. Implement interpolation via Vandermonde matrices. -- Done
// 3. Implement interpolation via Cauchy matrices.
// 4. Transpose the output for encode (i.e. output[i] should be a vector of all the i'th indexes).
// 5. Implement architecture-specific improvements.

#[warn(dead_code)]
fn encode_string() {
    let string = "Test string";
    println!("Bytes: {:?}", string.as_bytes());
    let encoding = Encoding::from_str("rs=6.4").expect("Should parse");
    let encoder = VandermondeEncoder::default();
    let stream = encoder
        .encode_bytes(encoding, &ExpLogField::default(), string.as_bytes())
        .expect(&format!(
            "Encoding did not work for byte stream: {}",
            string
        ));
    println!("Length: {:?}", stream.length);
    println!("Encoding: {:?}", stream.encoding);
    println!("Codes: {:?}", stream.codes);
}

fn main() {
    shamir(5, 3, 10);
    // Password: stunt bath gains cheer pecan haven date happy hatch swan
    unshamir(
        &[
            None,
            Some("wilt morse bring trout neon view pep ebay found cage"),
            None,
            Some("vowel shun lance bring crop ebay skip slush decal elves"),
            None,
            Some("neon foam open elbow bash award polo shack bath skip"),
        ],
        3,
    );
}
