# Usage

## Python

Install the extension from PyPI:

```console
python -m pip install fastbase91
```

See [Installation & compatibility](installation.md) for supported platforms, Python versions, and what to do without a pre-built wheel.

### One-shot encoding and decoding

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
decoded = fastbase91.decode(encoded)

assert encoded == b"TPwJh>A"
assert decoded == b"hello"
```

`encode(data)` and `decode(data, *, strict=False)` return `bytes`. Their input may be `bytes`, `bytearray`, or another compatible contiguous, one-dimensional byte buffer such as `memoryview`.

### Bytes, text, and line breaks

`encode()` returns `bytes` containing only printable ASCII bytes from the basE91 alphabet. To use encoded data as `str`, call `.decode("ascii")`. `decode()` accepts bytes-like objects, not `str`; encode text with `.encode("ascii")` first.

```python
text = fastbase91.encode(b"hello").decode("ascii")
assert text == "TPwJh>A"
assert fastbase91.decode(text.encode("ascii")) == b"hello"
```

The encoder does not insert line breaks. If a channel needs line-wrapped data, you must add the breaks yourself; lenient decoding skips them and strict decoding rejects them.

### Lenient and strict decoding

Lenient decoding is the default. It skips every byte outside the basE91 alphabet, including spaces, line breaks, NUL bytes, and bytes at `0x80` or above. The reference basE91 decoder uses the same treatment, so basE91 text that another tool wrapped across lines can be decoded directly.

```python
noisy = b"TP\nwJ h\x00>\xffA"
assert fastbase91.decode(noisy) == b"hello"
```

With `strict=True`, the first byte outside the alphabet raises `DecodeError`; a line break is outside the alphabet. `DecodeError` is a `ValueError` subclass whose `offset` and `byte` attributes identify that byte.

```python
try:
    fastbase91.decode(b"TPwJ\nh>A", strict=True)
except fastbase91.DecodeError as error:
    assert error.offset == 4
    assert error.byte == 0x0A
```

Use `strict=True` for untrusted input, or for a protocol where extra bytes indicate invalid data. Strict mode only checks whether each byte is in the alphabet: truncated or altered data can decode to different bytes without an error. basE91 is an encoding, not encryption, and provides no integrity protection. To detect accidental corruption, add a checksum at the application layer; to detect deliberate tampering, use a MAC or a digital signature, because anyone who can change the data can also recompute a plain checksum.

### Streaming

Stream data that does not fit in memory or arrives in chunks. Write each result as it is returned: memory used for payload data is bounded by the chunk size rather than the total input size. This example encodes `input.bin` to `encoded.b91`, then decodes it to `decoded.bin`; if `input.bin` is not present in the directory, it first writes 1 MB of sample data (place your own file at `input.bin` to process it).

```python
import filecmp
from pathlib import Path

if not Path("input.bin").exists():
    with Path("input.bin").open("wb") as sample:
        for start in range(0, 1_000_000, 1000):
            sample.write(bytes(i % 251 for i in range(start, start + 1000)))

chunk_size = 64 * 1024

encoder = fastbase91.Encoder()
with Path("input.bin").open("rb") as source, Path("encoded.b91").open("wb") as destination:
    while chunk := source.read(chunk_size):
        destination.write(encoder.update(chunk))
    destination.write(encoder.finish())

decoder = fastbase91.Decoder()
with Path("encoded.b91").open("rb") as source, Path("decoded.bin").open("wb") as destination:
    while chunk := source.read(chunk_size):
        destination.write(decoder.update(chunk))
    destination.write(decoder.finish())
assert filecmp.cmp("input.bin", "decoded.bin", shallow=False)
```

The final line compares the two files in small chunks with `filecmp.cmp(..., shallow=False)`, so the check itself does not read the entire files into memory.

Chunk boundaries do not change the result: concatenating streaming output gives the same bytes as one-shot `encode()` or `decode()`. `finish()` emits remaining bits that have not formed a complete group and marks the end of that message. After `finish()`, another `update()` or `finish()` on that object raises `ValueError`; use one `Encoder` or `Decoder` for each message.

Independently encoded messages must not be concatenated and decoded as one message: the result can contain incorrect bytes. Store and decode each message separately; when a shared container is needed, define and parse a delimiter or length before decoding.

### Handling decode errors

See Lenient and strict decoding above for one-shot `DecodeError` details. For `Decoder(strict=True)`, `DecodeError.offset` is measured from the start of the whole stream. A failed `update()` returns no output and leaves decoder state unchanged, while output from earlier `update()` calls remains valid. The failed call consumes no input, so after handling an invalid byte, remove or correct it in the entire chunk and pass that whole corrected chunk to the same decoder, including any valid bytes before the invalid byte, as in the example.

```python
decoder = fastbase91.Decoder(strict=True)
decoded = decoder.update(b"TPw")
try:
    decoded += decoder.update(b"J h>A")
except fastbase91.DecodeError as error:
    assert error.offset == 4
    assert error.byte == 0x20
decoded += decoder.update(b"Jh>A")
decoded += decoder.finish()
assert decoded == b"hello"
```

Unsupported buffer layouts (for example non-contiguous or multi-dimensional), or buffers whose items are not single bytes, raise `BufferError`. Calls after `finish()` raise `ValueError`, and passing `str` raises `TypeError`.

See [API reference](api-reference.md) for documented exceptions and when each occurs.

### Choosing one-shot or streaming

Use one-shot `encode()` or `decode()` when the data fits in memory; use streaming when it does not or when data arrives in chunks.

On GIL-enabled CPython, one-shot calls with input of at least 1,024 bytes release the GIL, allowing threads to run them concurrently. Smaller one-shot calls and all streaming `update()` and `finish()` calls hold the GIL. On free-threaded CPython, while the GIL stays disabled at runtime (`sys._is_gil_enabled()` returns `False`), independent calls and separate `Encoder` or `Decoder` objects can run concurrently; do not share one stream object across threads. [Free-threading](free-threading.md) explains what can turn the GIL back on.

Each streaming `update()` has fixed overhead. Very small chunks are noticeably slower; chunks of tens of KiB, such as 64 KiB, are sufficient. See [Benchmarks](benchmarks.md) for measured throughput of one-shot calls.

The package exposes its binding version as `fastbase91.__version__` and the linked Rust core version as `fastbase91.CORE_VERSION`.

## Rust

Add the core crate:

```console
cargo add fastbase91-core
```

See the complete Rust API documentation at [docs.rs](https://docs.rs/fastbase91-core); see [Installation & compatibility](installation.md) for features and the minimum Rust version.

### Allocating APIs

The default `std` feature enables `alloc`. With either `std` or `alloc`, one-shot calls return a `Vec<u8>`:

```rust
use fastbase91_core::{decode, encode, DecodeOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = encode(b"hello")?;
    assert_eq!(encoded, b"TPwJh>A");

    let decoded = decode(&encoded, DecodeOptions::default())?;
    assert_eq!(decoded, b"hello");
    Ok(())
}
```

The signatures are:

```rust
pub fn encode(input: &[u8]) -> Result<Vec<u8>, EncodeError>;
pub fn decode(
    input: &[u8],
    options: DecodeOptions,
) -> Result<Vec<u8>, DecodeError>;
```

`DecodeOptions::new()` and `DecodeOptions::default()` are lenient. To reject non-alphabet bytes, set the public field before decoding:

```rust
let mut options = fastbase91_core::DecodeOptions::new();
options.reject_non_alphabet = true;
```

### `no_std` without allocation

Disable default features when the target has neither `std` nor an allocator. Replace the version placeholder with the fastbase91-core release you want to use (see [crates.io](https://crates.io/crates/fastbase91-core)).

```toml
[dependencies]
fastbase91-core = { version = "<version>", default-features = false }
```

Use caller-provided buffers with `encode_into` and `decode_into`:

```rust
use fastbase91_core::{decode_into, encode_into, DecodeOptions};

let mut encoded = [0_u8; 10];
let encoded_len = encode_into(b"hello", &mut encoded).unwrap();
assert_eq!(&encoded[..encoded_len], b"TPwJh>A");

let mut decoded = [0_u8; 7];
let decoded_len = decode_into(
    &encoded[..encoded_len],
    &mut decoded,
    DecodeOptions::new(),
)
.unwrap();
assert_eq!(&decoded[..decoded_len], b"hello");
```

`encode_into` returns `Result<usize, OutputTooSmall>`. `decode_into` returns `Result<usize, DecodeError>`, whose variants include `OutputTooSmall` and `InvalidByte`. `max_encoded_len(input_len)` and `max_decoded_len(input_len)` return `Option<usize>` upper bounds for sizing output storage.

To use the allocating APIs in a `no_std` environment that does provide an allocator, enable only `alloc`. Replace the version placeholder with the fastbase91-core release you want to use (see [crates.io](https://crates.io/crates/fastbase91-core)).

```toml
fastbase91-core = { version = "<version>", default-features = false, features = ["alloc"] }
```

### Streaming core API

The core streaming types write into caller-provided slices:

```rust
Encoder::new()
Encoder::update(&mut self, input: &[u8], output: &mut [u8])
    -> Result<usize, OutputTooSmall>
Encoder::finish(self) -> ([u8; 2], usize)

Decoder::new(options: DecodeOptions)
Decoder::update(&mut self, input: &[u8], output: &mut [u8])
    -> Result<usize, DecodeError>
Decoder::finish(self) -> Result<Option<u8>, DecodeError>
```

The Rust `finish` methods consume the stream. For the encoder, copy the first `usize` bytes from the returned two-byte tail. For the decoder, append the returned byte when the result is `Some`.

```rust
use fastbase91_core::{
    max_decoded_len, max_encoded_len, DecodeOptions, Decoder, Encoder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const CHUNK: usize = 4096;
    const ENCODED_CAPACITY: usize = match max_encoded_len(CHUNK) {
        Some(n) => n,
        None => panic!("chunk is too large"),
    };
    const DECODED_CAPACITY: usize = match max_decoded_len(CHUNK) {
        Some(n) => n,
        None => panic!("chunk is too large"),
    };

    let input: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
    let mut encoder = Encoder::new();
    let mut encoded = Vec::new();
    for chunk in input.chunks(CHUNK) {
        let mut output = [0_u8; ENCODED_CAPACITY];
        let written = encoder.update(chunk, &mut output)?;
        encoded.extend_from_slice(&output[..written]);
    }
    let (tail, tail_len) = encoder.finish();
    encoded.extend_from_slice(&tail[..tail_len]);

    let mut decoder = Decoder::new(DecodeOptions::new());
    let mut decoded = Vec::new();
    for chunk in encoded.chunks(CHUNK) {
        let mut output = [0_u8; DECODED_CAPACITY];
        let written = decoder.update(chunk, &mut output)?;
        decoded.extend_from_slice(&output[..written]);
    }
    if let Some(tail) = decoder.finish()? {
        decoded.push(tail);
    }

    assert_eq!(decoded, input);
    Ok(())
}
```

The `Vec<u8>` values provide a `std` sink for the example; replacing their `extend_from_slice` calls with a sink keeps the update loop suitable for `no_std`. Buffers sized with `max_encoded_len(CHUNK)` and `max_decoded_len(CHUNK)` hold every `update()` result for input of at most `CHUNK` bytes in any stream state, so this example does not receive `OutputTooSmall`. With another buffer size, `update()` can report `OutputTooSmall`: the encoder returns it directly and the decoder wraps it as `DecodeError::OutputTooSmall`. Its `required()` method gives the required capacity. In that case, state and output remain unchanged, so retry with a larger buffer.

In strict mode, `InvalidByte { offset, .. }` reports the offset within the chunk passed to that `update()`, not the complete stream. Rust does not accumulate that offset. The decoder state remains unchanged on that error, but output written by that call is unspecified and must be discarded.

The crate version is available as `fastbase91_core::VERSION`.
