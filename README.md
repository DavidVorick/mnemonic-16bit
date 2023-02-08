# mnemonic-16bit

mnemonic-16bit is a mnemonic library that will take any binary data and convert it into a
phrase which is more human friendly. Each word of the phrase maps to 16 bits, where the first
10 bits are represented by one word from the seed15 dictionary, and the remaining 6 bits are
represented by a number between 0 and 63. If the number is '64', that signifies that the word
only represents 1 byte instead of 2. Only the final word of a phrase may use the numerical
suffix 64.

```rs
use mnemonic_16bit::{binary_to_phrase, phrase_to_binary};

fn main() {
    let my_data = [0u8; 2];
    let phrase = binary_to_phrase(&my_data); // "abbey0"
    let data = phrase_to_binary(&phrase).unwrap();
    assert!(data[..] == my_data[..]);
}
```
