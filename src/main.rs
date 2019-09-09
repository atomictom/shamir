#![feature(test)]

mod encoding;
mod finite_field;
mod polynomial;
mod rsstream;

use encoding::Encoding;
use finite_field::DirectField;
use rsstream::LagrangeInterpolationEncoder;
use rsstream::RSEncoder;
use std::str::FromStr;

fn main() {
    let string = "Test string";
    println!("Bytes: {:?}", string.as_bytes());
    let encoding = Encoding::from_str("rs=6.4").expect("Should parse");
    let stream = LagrangeInterpolationEncoder::encode_bytes(
        encoding,
        &DirectField::default(),
        string.as_bytes(),
    );
    println!("Length: {:?}", stream.length);
    println!("Encoding: {:?}", stream.encoding);
    println!("Codes: {:?}", stream.codes);
}
