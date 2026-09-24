"""Standard basE91 (Joachim Henke) binary-to-text codec.

Pure-Python baseline copied verbatim from the user's project
(pipdltk .../helper/base91.py) to serve as the benchmark baseline.
"""

from typing import Final

ALPHABET: Final[str] = (
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "abcdefghijklmnopqrstuvwxyz"
    "0123456789"
    '!#$%&()*+,./:;<=>?@[]^_`{|}~"'
)

NON_ALPHABET_MARKERS: Final[str] = "-'\\"

_DECODE_TABLE: Final[dict[str, int]] = {
    char: index for index, char in enumerate(ALPHABET)
}


def base91_encode(data: bytes) -> str:
    """Encode bytes to a basE91 string."""
    bit_buffer = 0
    bit_count = 0
    out: list[str] = []
    for byte in data:
        bit_buffer |= byte << bit_count
        bit_count += 8
        if bit_count > 13:
            value = bit_buffer & 8191
            if value > 88:
                bit_buffer >>= 13
                bit_count -= 13
            else:
                value = bit_buffer & 16383
                bit_buffer >>= 14
                bit_count -= 14
            out.append(ALPHABET[value % 91])
            out.append(ALPHABET[value // 91])
    if bit_count:
        out.append(ALPHABET[bit_buffer % 91])
        if bit_count > 7 or bit_buffer > 90:
            out.append(ALPHABET[bit_buffer // 91])
    return "".join(out)


def base91_decode(encoded: str) -> bytes:
    """Decode a basE91 string back to bytes."""
    value = -1
    bit_buffer = 0
    bit_count = 0
    out = bytearray()
    for char in encoded:
        index = _DECODE_TABLE.get(char)
        if index is None:
            continue
        if value < 0:
            value = index
        else:
            value += index * 91
            bit_buffer |= value << bit_count
            bit_count += 13 if (value & 8191) > 88 else 14
            while bit_count >= 8:
                out.append(bit_buffer & 255)
                bit_buffer >>= 8
                bit_count -= 8
            value = -1
    if value >= 0:
        out.append((bit_buffer | value << bit_count) & 255)
    return bytes(out)
