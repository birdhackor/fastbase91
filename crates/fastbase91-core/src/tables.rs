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
