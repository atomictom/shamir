// Convert between bytes and string words

use std::collections::HashMap;
use std::iter::FromIterator;

pub fn load_word_list(path: &str) -> Vec<String> {
    return std::fs::read_to_string(path)
        .expect(&format!("Could not read file at path {}", path))
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
}

#[allow(unused)]
pub fn to_words<'a, I: Iterator<Item = u8>>(bytes: I, wordlist: &[&'a str]) -> Vec<&'a str> {
    assert!(wordlist.len() >= 256);
    return bytes.map(|b| wordlist[b as usize]).collect();
}

pub fn from_words<'a, S: AsRef<str>, I: Iterator<Item = &'a str>>(
    words: I,
    wordlist: &'a [S],
) -> Vec<u8> {
    assert!(wordlist.len() >= 256);
    let words_index: HashMap<&'a str, u8> = HashMap::from_iter(
        wordlist
            .into_iter()
            .enumerate()
            .map(|(i, w)| (w.as_ref(), i as u8)),
    );
    return words.map(|w| words_index[w] as u8).collect();
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     const words: [String; 256] = [0..256].into_iter().map(|i| i.to_string());
//
//     #[test]
//     fn chunk_even_split_default() {
//         let bytes = &[1, 2, 3, 4, 5, 6, 7, 8];
//
//     }
// }
