pub(crate) const ALPHABET: [u8; 91] =
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";

const fn decode_table() -> [u8; 256] {
    let mut table = [91; 256];
    let mut index = 0;
    while index < ALPHABET.len() {
        table[ALPHABET[index] as usize] = index as u8;
        index += 1;
    }
    table
}

pub(crate) const DECODE_TABLE: [u8; 256] = decode_table();

const fn decode_table_ff() -> [u8; 256] {
    let mut table = [0xFF; 256];
    let mut index = 0;
    while index < ALPHABET.len() {
        table[ALPHABET[index] as usize] = index as u8;
        index += 1;
    }
    table
}

/// Same mapping as `DECODE_TABLE`, but the non-alphabet sentinel is `0xFF`, so
/// eight lookups can be validated with one OR-reduction and a bit test.
pub(crate) static DECODE_FF: [u8; 256] = decode_table_ff();

const fn encode_pairs() -> [[u8; 2]; 8281] {
    let mut table = [[0; 2]; 8281];
    let mut value = 0;
    while value < table.len() {
        table[value] = [ALPHABET[value % 91], ALPHABET[value / 91]];
        value += 1;
    }
    table
}

/// Every pair value in `0..91 * 91` mapped to its two symbols, low digit first.
pub(crate) static ENCODE_PAIRS: [[u8; 2]; 8281] = encode_pairs();
