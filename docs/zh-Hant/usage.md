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

### 位元組、文字與換行

`encode()` 回傳的 `bytes` 只含 basE91 字母表裡的可列印 ASCII 位元組。若要將編碼資料當成 `str` 使用，呼叫 `.decode("ascii")`。`decode()` 接受 bytes-like 物件，不接受 `str`；請先用 `.encode("ascii")` 把文字轉成位元組。

```python
text = fastbase91.encode(b"hello").decode("ascii")
assert text == "TPwJh>A"
assert fastbase91.decode(text.encode("ascii")) == b"hello"
```

Encoder 不會插入換行。通道若需要分行，需要由你自行加入換行；寬鬆解碼會略過換行，嚴格解碼則會拒絕換行。

### 寬鬆與嚴格解碼

寬鬆解碼是預設值，會略過所有不在 basE91 字母表裡的位元組，包括空白、換行、NUL 位元組，以及 `0x80` 以上的位元組。basE91 原作者的參考解碼器也採相同處理方式，因此可直接解碼由其他工具分行過的 basE91 文字。

```python
noisy = b"TP\nwJ h\x00>\xffA"
assert fastbase91.decode(noisy) == b"hello"
```

設定 `strict=True` 時，第一個字母表外的位元組會拋出 `DecodeError`；換行也屬於字母表外的位元組。`DecodeError` 是 `ValueError` 的子類別，`offset` 與 `byte` 屬性會指出該位元組。

```python
try:
    fastbase91.decode(b"TPwJ\nh>A", strict=True)
except fastbase91.DecodeError as error:
    assert error.offset == 4
    assert error.byte == 0x0A
```

處理未信任的輸入，或協定中多出的位元組就代表資料無效時，請使用 `strict=True`。嚴格模式只檢查每個位元組是否屬於字母表：截斷或被改掉的資料仍可能解出不同的位元組而不報錯。basE91 是編碼，不是加密，也不提供完整性保護。只需偵測意外毀損時，可在應用程式層加入 checksum；要偵測蓄意竄改，必須使用 MAC 或數位簽章，因為能改動資料的人也能重新算出一般的 checksum。

### 串流操作

資料放不進記憶體或分段抵達時，請用串流。每次拿到結果就寫出去，承載資料所用的記憶體會受區塊大小限制，不隨總輸入大小增加。

```python
from pathlib import Path

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

```

`decoded.bin` 與 `input.bin` 逐位元組相同。

切塊位置不影響結果：把串流輸出接起來，會得到與一次性 `encode()` 或 `decode()` 相同的位元組。`finish()` 會送出尚未湊成完整一組的剩餘位元，並標示該則訊息結束。呼叫 `finish()` 後，若再對同一個物件呼叫 `update()` 或 `finish()`，會拋出 `ValueError`；每則訊息各用一個 `Encoder` 或 `Decoder`。

各自編碼的訊息不得接在一起當成一則訊息解碼，否則結果可能含有錯誤的位元組。請分開儲存、分開解碼；需要共用容器時，在解碼前自行定義並解析分隔符號或長度。

### 處理解碼錯誤

一次性 `DecodeError` 的說明請見前面的「寬鬆與嚴格解碼」。對 `Decoder(strict=True)` 而言，`DecodeError.offset` 是從整個串流的開頭起算。失敗的 `update()` 不會回傳輸出，且不會改變 decoder 狀態；先前 `update()` 已回傳的輸出仍然有效。失敗的呼叫不會消耗任何輸入，因此處理無效位元組時，要先在整個區塊中移除或修正它，再把修正後的整個區塊交給同一個 decoder；這也包含無效位元組之前的有效位元組，如本例。

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

不支援的 buffer 形式（例如非連續或多維），或元素不是單一位元組的 buffer，會拋出 `BufferError`。`finish()` 後的呼叫會拋出 `ValueError`，傳入 `str` 則會拋出 `TypeError`。

### 選擇一次性或串流

資料放得進記憶體時，使用一次性 `encode()` 或 `decode()`；資料放不進去或分段抵達時，使用串流。

在有 GIL 的 CPython 上，輸入至少為 1,024 位元組的一次性呼叫會放掉 GIL，讓多條執行緒可並行執行。較小的一次性呼叫，以及所有串流的 `update()` 與 `finish()` 呼叫都握著 GIL。自由執行緒 CPython 沒有 GIL，所以各自獨立的呼叫與分開的 `Encoder` 或 `Decoder` 物件可以並行；不要讓多條執行緒共用同一個串流物件。詳情見[自由執行緒](free-threading.md)。

每次串流 `update()` 都有固定成本。極小區塊會明顯變慢；幾十 KiB 的區塊，例如 64 KiB，就足夠。一次性呼叫的實測吞吐量請見[效能測試](benchmarks.md)。

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

範例中的 `Vec<u8>` 是 `std` sink；把 `extend_from_slice` 呼叫改成寫入自己的 sink，`update()` 迴圈就可用於 `no_std`。以 `max_encoded_len(CHUNK)` 與 `max_decoded_len(CHUNK)` 配置的 buffer，對任何串流狀態下、至多 `CHUNK` 位元組輸入的每次 `update()` 都足夠，所以範例不會收到 `OutputTooSmall`。若使用其他大小的 buffer，`update()` 可能回傳 `OutputTooSmall`；其 `required()` 方法會指出所需容量。這種情況下狀態與輸出都不變，可改用較大的 buffer 重試。

嚴格模式的 `InvalidByte { offset, .. }` 所報的 `offset` 是這次傳給 `update()` 的區塊內位移，不是整個串流。Rust 不會累計該位移。該錯誤發生時 decoder 狀態不變，但該次呼叫已寫入 `output` 的內容未定，必須丟棄。

Crate 版本可由 `fastbase91_core::VERSION` 取得。
