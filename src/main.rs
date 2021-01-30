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
    NoCommand,
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
    match args.next() {
        None => {
            println!("Expected a command from ['generate', 'restore']");
            exit(ExitCode::NoCommand);
        }
        Some(command) => {
            let remaining_args: Vec<String> = Vec::from_iter(args);
            match &command[..] {
                "generate" => generate(parse_options(&remaining_args)),
                "restore" => restore(parse_options(&remaining_args)),
                _ => {
                    println!("Expected a command from ['generate', 'restore']");
                    exit(ExitCode::WrongCommand);
                }
            };
        }
    }
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
    if options.total.is_some() && options.required.is_some() && options.total <= options.required {
        println!("Total shards must be larger than required shards (the secret uses a shard).");
        println!("Options: {:?}", options);
        exit(ExitCode::WrongShards);
    }

    return options;
}

fn generate(options: Options) {
    println!("-- Generating secret and shards... --");
    let required = match options.required {
        None => {
            println!("No --required flag, using default of 3.");
            3
        }
        Some(required) => required,
    };
    let total = match options.total {
        None => {
            println!("No --total flag, using default of --required + 1.");
            required + 1
        }
        Some(total) => total,
    };
    let words = match options.words {
        None => {
            println!("No --words flag, using default of 10.");
            3
        }
        Some(words) => words,
    };
    let shards: Vec<String> = shamir(total, required, words);

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
