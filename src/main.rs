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
use std::env::args;
use std::io;
use std::io::Write;
use std::iter::FromIterator;

enum ExitCode {
    Success = 0,
    WrongCommand,
    WrongShards,
    UnrecognizedArgument,
}

fn exit(code: ExitCode) -> ! {
    std::process::exit(code as i32);
}

fn main() {
    let mut args = args();
    let _executable: String = args.next().unwrap_or("shamir".to_owned());
    let command: String = args
        .next()
        .expect("Expected a command from 'generate' or 'restore'");
    let remaining_args: Vec<String> = Vec::from_iter(args);
    match &command[..] {
        "generate" => generate(parse_options(&remaining_args)),
        "restore" => restore(parse_options(&remaining_args)),
        _ => {
            println!("Expected a command from ['generate', 'restore']");
            exit(ExitCode::WrongCommand);
        }
    };
    exit(ExitCode::Success);
}

#[derive(Debug, PartialEq)]
struct Options {
    total: Option<usize>,
    required: Option<usize>,
    words: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            total: None,
            required: None,
            words: None,
        }
    }
}

fn parse_options(args: &Vec<String>) -> Options {
    let mut options: Options = Default::default();
    let mut index = 0;
    loop {
        if args.len() - index < 2 {
            break;
        }
        match &args[index][..] {
            "--required" => {
                options.required = Some(
                    args[index + 1]
                        .parse::<usize>()
                        .expect("Could not parse the --required option"),
                );
            }
            "--total" => {
                options.total = Some(
                    args[index + 1]
                        .parse::<usize>()
                        .expect("Could not parse the --total option"),
                );
            }
            "--words" => {
                options.words = Some(
                    args[index + 1]
                        .parse::<usize>()
                        .expect("Could not parse the --words option"),
                );
            }
            _ => {
                println!("Unrecognized argument {}", args[index]);
                exit(ExitCode::UnrecognizedArgument);
            }
        }
        index += 2;
    }
    if (options.total.is_none() && options.required.is_none()) || options.total > options.required {
        println!("Total shards must be larger than required shards (the secret is 1 shard).");
        println!("Options: {:?}", options);
        exit(ExitCode::WrongShards);
    }

    return options;
}

fn generate(options: Options) {
    println!("-- Generating secret and shards... --");
    let shards: Vec<String> = shamir(
        options.total.unwrap_or(6),
        options.required.unwrap_or(3),
        options.words.unwrap_or(10),
    );
    for (i, s) in shards.iter().enumerate() {
        if i == 0 {
            println!("Secret: {}", s);
        } else {
            println!("Shard {}: {}", i, s);
        }
    }
}

fn prompt(msg: &str) -> io::Result<String> {
    let mut input: String = String::new();
    let handle = std::io::stdin();
    print!("{}", msg);
    io::stdout().flush()?;
    let res = handle.read_line(&mut input);
    match res {
        Err(e) => {
            println!("Could not read line: {}", e);
            return Err(e);
        }
        Ok(_) => return Ok(input),
    }
}

fn restore(options: Options) {
    println!("-- Restoring the secret... --");
    let total = match options.total {
        None => prompt("How many total shards are there?: ")
            .expect("Could not determine the total number of shards.")
            .trim()
            .parse::<usize>()
            .expect("Could not convert total shards to integer."),
        Some(n) => n,
    };
    let required = match options.required {
        None => prompt("How many required shards are there?: ")
            .expect("Could not determine the number of required shards.")
            .trim()
            .parse::<usize>()
            .expect("Could not convert required shards to integer."),
        Some(n) => n,
    };

    println!(
        "You will be prompted to enter {} shards (in any order)...",
        required
    );

    let mut some_shards: Vec<String> = Vec::new();
    for i in 0..required {
        let shard = prompt(format!("Input shard {}: ", i).as_str())
            .expect(format!("Could not read shard {}", i).as_str());
        some_shards.push(shard);
    }

    unshamir(&some_shards, required, total + 1);
}
