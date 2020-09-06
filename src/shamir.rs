use crate::encoder::RSEncoder;
use crate::encoder::RSStream;
use crate::encoder::VandermondeEncoder;
use crate::encoding::Encoding;
use crate::finite_field::ExpLogField;
use crate::words;
use rand::Rng;

fn gen_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    return (0..length).map(|_| rng.gen()).collect();
}

pub fn shamir(shards: usize, required: usize, length: usize) -> Vec<String> {
    assert!(shards >= required);
    println!("Shards: {}, required: {}", shards, required);
    let wordlist = words::load_word_list("./assets/wordlist256.txt");

    let encoding = Encoding {
        data_chunks: required as u8,
        code_chunks: (shards - required + 1) as u8,
    };
    let encoder = VandermondeEncoder::default();
    let field = ExpLogField::default();

    let mut phrases: Vec<Vec<&str>> = (0..shards + 1)
        .map(|_| Vec::with_capacity(length))
        .collect();
    for _ in 0..length {
        let bytes = gen_random_bytes(required);
        let stream = encoder
            .encode_bytes(encoding, &field, &bytes[..])
            .expect(&format!(
                "Encoding did not work for byte stream: {:?}",
                &bytes
            ));
        for (j, b) in stream.codes[0].iter().enumerate() {
            phrases[j].push(&wordlist[*b as usize]);
        }
    }

    let strings: Vec<String> = phrases.into_iter().map(|words| words.join(" ")).collect();
    for (i, s) in strings.iter().enumerate() {
        if i == 0 {
            println!("Password: {}", s);
        } else {
            println!("Shard {}: {}", i, s);
        }
    }
    return strings;
}

// Note that phrases is positional
pub fn unshamir(phrases: &[Option<&str>], required: usize) -> String {
    let wordlist: Vec<String> = words::load_word_list("./assets/wordlist256.txt");
    let valid: Vec<bool> = phrases.iter().map(|x| x.is_some()).collect();
    println!("Valid: {:?}", valid);
    let words: Vec<Vec<u8>> = phrases
        .into_iter()
        .map(|x| match x {
            None => Vec::new(),
            Some(s) => words::from_words(s.split(" "), &wordlist[..]),
        })
        .collect();
    let length: usize = words.iter().map(|x| x.len()).max().unwrap_or(0);
    println!("Length: {}", length);
    assert!(words.iter().all(|x| x.len() == 0 || x.len() == length));
    let codes: Vec<Vec<u8>> = (0..length)
        .map(|i| {
            (0..words.len())
                .map(|j| {
                    if words[j].len() == length {
                        words[j][i]
                    } else {
                        0
                    }
                })
                .collect()
        })
        .collect();
    let encoding = Encoding {
        data_chunks: required as u8,
        code_chunks: (phrases.len() - required) as u8,
    };
    println!("Encoding: {:?}", encoding);
    let encoder = VandermondeEncoder::default();
    let field = ExpLogField::default();

    let mut password_words: Vec<&str> = Vec::with_capacity(length);
    for chunk in codes {
        let stream = RSStream {
            length: required,
            encoding: encoding,
            codes: vec![chunk],
            valid: valid.clone(),
        };
        match encoder.decode_bytes(&stream, &field) {
            Ok(data) => password_words.push(&wordlist[data[0] as usize]),
            Err(e) => panic!("Got an error {} while decoding.", e),
        };
    }
    let password = password_words.join(" ");
    println!("Shards: {}, required: {}", phrases.len(), required);
    println!("Password: {}", password);
    return password;
}
