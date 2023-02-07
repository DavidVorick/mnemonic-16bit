#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]
#![deny(unused_mut)]

//! mnemonic-16bit is a mnemonic library that will take any binary data and convert it into a
//! phrase which is more human friendly. Each word of the phrase maps to 16 bits, where the first
//! 10 bits are represented by one word from the seed15 dictionary, and the remaining 6 bits are
//! represented by a number between 0 and 63. If the number is '64', that signifies that the word
//! only represents 1 byte instead of 2. Only the final word of a phrase may use the numerical
//! suffix 64.
//!
//! ```
//! use mnemonic_16bit::{binary_to_phrase, phrase_to_binary};
//!
//! fn main() {
//!     let my_data = [0u8; 2];
//!     let phrase = binary_to_phrase(&my_data); // "abbey0"
//!     let data = phrase_to_binary(&phrase).unwrap();
//!     assert!(data[..] == my_data[..]);
//! }
//! ```

use anyhow::{bail, Context, Error, Result};
use seed15::dictionary::{DICTIONARY, DICTIONARY_UNIQUE_PREFIX};

/// binary_to_phrase will convert a binary string to a phrase.
pub fn binary_to_phrase(data: &[u8]) -> String {
    // Base case, no data means no mnemonic.
    let mut phrase = "".to_string();
    if data.len() == 0 {
        return phrase;
    }

    // Parse out all of the even-numbered bytes.
    let mut i = 0;
    while i+1 < data.len() {
        // Determine the dictionary offset.
        let mut word_index = data[i] as u16;
        word_index *= 4;
        let word_bits = data[i+1] / 64;
        word_index += word_bits as u16;
        let word = DICTIONARY[word_index as usize];

        // Determine the accompanying number.
        let num = data[i+1] % 64;

        // Compose the word into the phrase.
        if phrase.len() != 0 {
            phrase += " ";
        }
        phrase += word;
        phrase += &format!("{}", num);
        i += 2;
    }

    // Parse out the final word.
    if data.len() % 2 == 1 {
        let word = DICTIONARY[data[i] as usize];
        if phrase.len() != 0 {
            phrase += " ";
        }
        phrase += word;
        phrase += "64";
    }

    phrase
}

/// dict_index returns the index of the word in the dictionary. An error is returned if the word is
/// not found.
fn dict_index(word: &str) -> Result<u16, Error> {
    // Only the prefix matters.
    let word = &word[..DICTIONARY_UNIQUE_PREFIX];
    for i in 0..DICTIONARY.len() {
        if DICTIONARY[i][..DICTIONARY_UNIQUE_PREFIX] == *word {
            return Ok(i as u16);
        }
    }
    bail!("word is not in dictionary");
}

/// phrase_to_binary is the inverse of binary_to_phrase, it will take a mnonmic-16bit phrase and
/// parse it into a set of bytes.
pub fn phrase_to_binary(phrase: &str) -> Result<Vec<u8>, Error> {
    if phrase == "" {
        return Ok(vec![0u8; 0]);
    }

    // Parse the words one at a time.
    let mut finalized = false;
    let mut result: Vec<u8> = Vec::new();
    let words = phrase.split(" ");
    for word in words {
        if finalized {
            bail!("only the last word may contain the number '64'");
        }

        // Make sure there are only numeric characters at the end of the string.
        let mut digits = 0;
        for c in word.chars() {
            if digits > 0 && !c.is_ascii_digit() {
                bail!("number must appear as suffix only");
            }
            if digits > 1 {
                bail!("number must be at most 2 digits");
            }
            if c.is_ascii_digit() {
                digits += 1;
            }
        }
        if digits == 0 {
            bail!("word must have a numerical suffix");
        }

        // We have validated the word, now we need to parse the bytes. We start with the numerical
        // suffix because that indicates whether we are pulling 8 bits from the word or 10.
        let numerical_suffix;
        if digits == 1 {
            numerical_suffix = &word[word.len()-1..];
        } else {
            numerical_suffix = &word[word.len()-2..];
        }

        // Parse the rest of the data based on whether the final digit is 64 or less.
        if numerical_suffix == "64" {
            finalized = true;
            let word_index = dict_index(word).context(format!("invalid word {} in phrase", word))?;
            if word_index > 255 {
                bail!("final word is invalid, needs to be among the first 255 words in the dictionary");
            }
            result.push(word_index as u8);
        } else {
            let mut bits = dict_index(word).context(format!("invalid word {} in phrase", word))?;
            bits *= 64;
            let numerical_bits: u16 = numerical_suffix.parse().unwrap();
            if numerical_bits > 64 {
                bail!("numerical suffix must have a value [0, 64]");
            }
            bits += numerical_bits;
            result.push((bits / 256) as u8);
            result.push((bits % 256) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use userspace_rng::Csprng;
    use rand_core::RngCore;

    #[test]
    // Try a bunch of binary arrays and see that they all correctly convert into phrases and then
    // back into the same binary.
    fn check_seed_phrases() {
        // Try empty array.
        let basic = [0u8; 0];
        let phrase = binary_to_phrase(&basic);
        let result = phrase_to_binary(&phrase).unwrap();
        assert!(basic[..] == result[..]);

        // Try all possible 1 byte values.
        for i in 0..=255 {
            let basic = [i as u8; 1];
            let phrase = binary_to_phrase(&basic);
            let result = phrase_to_binary(&phrase).unwrap();
            assert!(basic[..] == result[..]);
        }

        // Try zero values for all possible array sizes 0-255.
        for i in 0..=255 {
            let basic = vec![0u8; i];
            let phrase = binary_to_phrase(&basic);
            let result = phrase_to_binary(&phrase).unwrap();
            assert!(basic[..] == result[..]);
        }

        // Try random data for all array sizes 0-255, 8 variations each size.
        let mut rng = Csprng {};
        for _ in 0..8 {
            for i in 0..=255 {
                let mut basic = vec![0u8; i];
                rng.fill_bytes(&mut basic);
                let phrase = binary_to_phrase(&basic);
                let result = phrase_to_binary(&phrase).unwrap();
                assert!(basic[..] == result[..]);
            }
        }

        // Try all possible 2 byte values.
        for i in 0..=255 {
            for j in 0..=255 {
                let mut basic = [0u8; 2];
                basic[0] = i;
                basic[1] = j;
                let phrase = binary_to_phrase(&basic);
                let result = phrase_to_binary(&phrase).unwrap();
                assert!(basic[..] == result[..]);
            }
        }
    }

    #[test]
    // Check a variety of invalid phrases.
    fn check_bad_phrases() {
        phrase_to_binary("a").unwrap_err();
        phrase_to_binary("a64").unwrap_err();
        phrase_to_binary("abbey").unwrap_err();
        phrase_to_binary("abbey65").unwrap_err();
        phrase_to_binary("yacht64").unwrap_err();
        phrase_to_binary("sugar21 ab55 mob32").unwrap_err();
        phrase_to_binary("sugar21 toffee mob32").unwrap_err();

        // This one should work even though we trucated the words.
        phrase_to_binary("sug21 tof21 mob32").unwrap();
    }
}
