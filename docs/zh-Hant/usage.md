# 使用方式

## Python

從 PyPI 安裝擴充套件：

```console
python -m pip install fastbase91
```

### One-shot 編解碼

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
decoded = fastbase91.decode(encoded)

assert encoded == b"TPwJh>A"
assert decoded == b"hello"
```

`encode(data)` 與 `decode(data, *, strict=False)` 都回傳 `bytes`。輸入可以是 `bytes`、`bytearray`，或其他相容的連續一維位元組 buffer，例如 `memoryview`。

解碼預設採寬鬆模式，會忽略 basE91 字母表以外的位元組：

```python
assert fastbase91.decode(b"TPw Jh>A") == b"hello"
```

若必須拒絕非字母表位元組，請設定 `strict=True`。`DecodeError` 是 `ValueError` 的子類別，`offset` 與 `byte` 屬性會指出有問題的輸入：

```python
try:
    fastbase91.decode(b"TPw Jh>A", strict=True)
except fastbase91.DecodeError as error:
    assert error.offset == 3
    assert error.byte == 0x20
```

### 串流操作

每個獨立串流都應使用新的 encoder 或 decoder；收集每次 `update()` 的結果，最後再附加 `finish()` 的結果：

```python
encoder = fastbase91.Encoder()
encoded = encoder.update(b"hel")
encoded += encoder.update(b"lo")
encoded += encoder.finish()
assert encoded == b"TPwJh>A"

decoder = fastbase91.Decoder(strict=False)
decoded = decoder.update(encoded[:3])
decoded += decoder.update(encoded[3:])
decoded += decoder.finish()
assert decoded == b"hello"
```

`Encoder.update(data)` 與 `Decoder.update(data)` 會回傳該區塊目前可產生的輸出。`finish()` 會輸出尾端資料並關閉 Python 串流物件；之後若對同一物件再次呼叫 `update()` 或 `finish()`，會拋出 `ValueError`。傳入 `strict=True` 建立 `Decoder`，即可拒絕第一個非字母表位元組。

套件版本可由 `fastbase91.__version__` 取得；所連結 Rust core 的版本則是 `fastbase91.CORE_VERSION`。

## Rust

加入 core crate：

```console
cargo add fastbase91-core
```

### 會配置記憶體的 API

預設的 `std` feature 會啟用 `alloc`。啟用 `std` 或 `alloc` 時，one-shot 呼叫會回傳 `Vec<u8>`：

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

方法簽章如下：

```rust
pub fn encode(input: &[u8]) -> Result<Vec<u8>, EncodeError>;
pub fn decode(
    input: &[u8],
    options: DecodeOptions,
) -> Result<Vec<u8>, DecodeError>;
```

`DecodeOptions::new()` 與 `DecodeOptions::default()` 都是寬鬆模式。若要拒絕非字母表位元組，請在解碼前設定公開欄位：

```rust
let mut options = fastbase91_core::DecodeOptions::new();
options.reject_non_alphabet = true;
```

### 不配置記憶體的 `no_std`

若目標環境既沒有 `std` 也沒有 allocator，請關閉預設 feature。把版本佔位字串換成你要使用的 fastbase91-core 版本（見 [crates.io](https://crates.io/crates/fastbase91-core)）：

```toml
[dependencies]
fastbase91-core = { version = "<version>", default-features = false }
```

透過 `encode_into` 與 `decode_into` 使用呼叫端提供的 buffer：

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

`encode_into` 回傳 `Result<usize, OutputTooSmall>`。`decode_into` 回傳 `Result<usize, DecodeError>`，其 variant 包含 `OutputTooSmall` 與 `InvalidByte`。`max_encoded_len(input_len)` 與 `max_decoded_len(input_len)` 會回傳 `Option<usize>` 的輸出空間上限，供配置 buffer 時使用。

若 `no_std` 環境有 allocator 並需要會配置記憶體的 API，可只啟用 `alloc`。把版本佔位字串換成你要使用的 fastbase91-core 版本（見 [crates.io](https://crates.io/crates/fastbase91-core)）：

```toml
fastbase91-core = { version = "<version>", default-features = false, features = ["alloc"] }
```

### Core 串流 API

Core 串流型別會寫入呼叫端提供的 slice：

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

Rust 的 `finish` 方法會消耗串流。Encoder 回傳兩個位元組的尾端陣列與長度，請複製其中前 `usize` 個位元組；decoder 回傳 `Some` 時，則附加其中的位元組。

Crate 版本可由 `fastbase91_core::VERSION` 取得。
