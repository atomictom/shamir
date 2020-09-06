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
use encoder::RSStream;
use encoder::VandermondeEncoder;
use encoding::Encoding;
use finite_field::ExpLogField;
use std::iter;
use std::str::FromStr;

// TODO:
//
// 1. Implement fast direct inverses used the extended euclidean algorithm. -- too hard, not worth
// 2. Implement interpolation via Vandermonde matrices. -- Done
// 3. Implement interpolation via Cauchy matrices.
// 4. Transpose the output for encode (i.e. output[i] should be a vector of all the i'th indexes).
// 5. Implement architecture-specific improvements.

#[allow(unused)]
fn encode_string(s: &str) -> RSStream {
    println!("Encoding: {:?}", s);
    println!("Bytes: {:?}", s.as_bytes());
    let encoding = Encoding::from_str("rs=6.4").expect("Should parse");
    let encoder = VandermondeEncoder::default();
    let stream = encoder
        .encode_bytes(encoding, &ExpLogField::default(), s.as_bytes())
        .expect(&format!("Encoding did not work for byte stream: {}", s));
    println!("Length: {:?}", stream.length);
    println!("Encoding: {:?}", stream.encoding);
    println!("Codes: {:?}", stream.codes);
    return stream;
}

#[allow(unused)]
fn decode_string(stream: &RSStream) -> String {
    let encoder = VandermondeEncoder::default();
    let bytes = encoder
        .decode_bytes(stream, &ExpLogField::default())
        .expect(&format!(
            "Encoding did not work for RS stream: {:?}",
            stream
        ));
    return match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => format!("Got utf8 parsing error: {:?}", e),
    };
}

fn destroy_column(stream: &mut RSStream, column: usize) {
    println!("Destroying data in column {}", column);
    stream.valid[column] = false;
    stream
        .codes
        .iter_mut()
        .for_each(|row: &mut Vec<u8>| row[column] = 0);
}

fn main() {
    println!("-- RS Encoding -- ");
    let mut stream = encode_string("Test string");
    stream.valid = iter::repeat(true).take(10).collect();

    println!("\n");
    println!("-- RS Decoding -- ");
    // 10 columns total, 6 data and 4 codes. We can destroy 4 and recover. Let's destroy 2 data and
    // 2 codes.
    destroy_column(&mut stream, 0);
    destroy_column(&mut stream, 1);
    destroy_column(&mut stream, 8);
    destroy_column(&mut stream, 9);
    println!("Damaged stream: {:?}", stream);
    let string = decode_string(&stream);
    println!("Recovered string: {:?}", string);

    println!("\n");
    println!("-- Failed RS Decoding -- ");
    // Let's destroy one more column (but then say it's valid, otherwise we'll just get an error).
    destroy_column(&mut stream, 2);
    stream.valid[2] = true; // Sure it is...
    println!("Damaged stream: {:?}", stream);
    let string = decode_string(&stream);
    println!("Recovered string: {:?}", string);

    println!("\n");
    println!("-- Shamiring it up --");
    // Generates 6 total shards, the 0th shard is the password.
    let shards: Vec<String> = shamir(5, 3, 10);
    // Keep only the odd shards (half of them)
    let some_shards: Vec<Option<&str>> = shards
        .iter()
        .enumerate()
        .map(|(i, s)| if i % 2 == 1 { Some(s.as_str()) } else { None })
        .collect();

    // Password: stunt bath gains cheer pecan haven date happy hatch swan
    // unshamir(
    //     &[
    //         None,
    //         Some("wilt morse bring trout neon view pep ebay found cage"),
    //         None,
    //         Some("vowel shun lance bring crop ebay skip slush decal elves"),
    //         None,
    //         Some("neon foam open elbow bash award polo shack bath skip"),
    //     ],
    //     3,
    // );

    println!("\n");
    println!("-- Unshamiring it down --");
    let pretty: Vec<String> = some_shards
        .iter()
        .enumerate()
        .map(|(i, x)| format!("\tShard {}: {:?}", i, x))
        .collect();
    println!("Input shards: \n{}", pretty.join("\n"));
    unshamir(&some_shards[..], 3);
}
