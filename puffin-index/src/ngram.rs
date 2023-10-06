use std::fmt;

const NGRAM_SIZE: usize = 3;

const RUNE_MASK: u64 = (1 << 21) - 1;

/// Ngram is an encoded representation of N-Gram - a series of N adjacent chars.
/// This implementation is a trigram; it uses N = 3.
/// We use u64 under the hood for optimization while keeping enough bytes to account
/// for UNICODE characters.
// Heavily inspired by https://github.com/sourcegraph/zoekt.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Ngram(u64);

impl Ngram {
    pub fn to_runes(&self) -> [char; NGRAM_SIZE] {
        [
            char::from_u32(((self.0 >> 42) & RUNE_MASK) as u32).unwrap(),
            char::from_u32(((self.0 >> 21) & RUNE_MASK) as u32).unwrap(),
            char::from_u32(((self.0) & RUNE_MASK) as u32).unwrap(),
        ]
    }

    #[allow(dead_code)]
    pub fn to_bytes(&self) -> [u8; NGRAM_SIZE] {
        let rune_chars = self.to_runes();
        [
            rune_chars[0] as u8,
            rune_chars[1] as u8,
            rune_chars[2] as u8,
        ]
    }
}

impl AsRef<[u8]> for Ngram {
    fn as_ref(&self) -> &[u8] {
        let bytes = self.0.to_le_bytes();
        unsafe { std::slice::from_raw_parts(bytes.as_ptr(), std::mem::size_of::<u64>()) }
    }
}

impl fmt::Display for Ngram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chars = self.to_runes();
        write!(f, "{}", chars.iter().collect::<String>())
    }
}

impl From<u64> for Ngram {
    fn from(val: u64) -> Self {
        Ngram(val)
    }
}

impl From<&[u8]> for Ngram {
    fn from(val: &[u8]) -> Self {
        let mut chars = [0u8; NGRAM_SIZE];
        chars.copy_from_slice(&val[0..NGRAM_SIZE]);
        let rune_chars: [char; NGRAM_SIZE] = [
            char::from(chars[0]),
            char::from(chars[1]),
            char::from(chars[2]),
        ];
        Self::from(rune_chars)
    }
}

impl From<Ngram> for String {
    fn from(val: Ngram) -> Self {
        String::from_utf8(val.as_ref().to_vec()).unwrap()
    }
}

impl From<&str> for Ngram {
    fn from(val: &str) -> Self {
        Ngram::from(val.as_bytes())
    }
}

impl From<[char; NGRAM_SIZE]> for Ngram {
    fn from(val: [char; NGRAM_SIZE]) -> Self {
        Self::from((val[0] as u64) << 42 | (val[1] as u64) << 21 | (val[2] as u64))
    }
}

pub fn split_ngrams(s: &str) -> Vec<(Ngram, u64)> {
    let mut rune_gram = ['\0'; NGRAM_SIZE];
    let mut rune_count = 0;
    let mut result = Vec::new();

    for r in s.chars() {
        rune_gram[0] = rune_gram[1];
        rune_gram[1] = rune_gram[2];
        rune_gram[2] = r;
        rune_count += 1;

        if rune_count < NGRAM_SIZE {
            continue;
        }

        let ng = Ngram::from(rune_gram);
        result.push((ng, result.len() as u64));
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::ngram::*;

    #[test]
    fn ngram_from_runes() {
        assert_eq!(
            Ngram::from(['T', 'e', 's']),
            // 'T' == 84 == 0b01010100
            // 'e' == 101 == 0b01100101
            // 's' == 115 == 0b01110011
            Ngram::from(0b1010100000000000000001100101000000000000001110011)
        );

        assert_eq!(
            Ngram::from(['💩', 'e', 's']),
            // '💩' == 128169 == 0b11111010010101001
            // 'e' == 101 == 0b01100101
            // 's' == 115 == 0b01110011
            Ngram::from(0b11111010010101001000000000000001100101000000000000001110011)
        );
    }

    #[test]
    fn test_split_ngrams() {
        assert_eq!(
            vec![(Ngram::from("Tes"), 0), (Ngram::from("est"), 1),],
            split_ngrams("Test")
        );
    }
}
