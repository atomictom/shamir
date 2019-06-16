use std::result::Result;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
pub struct Encoding {
    pub data_chunks: u8,
    pub code_chunks: u8,
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
            .unwrap()
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
