# API 參考

## Python

下列 Python API 是套件的型別 stub（`fastbase91/__init__.pyi`）原文。建置網站時，這個檔案會直接從原始碼樹嵌入，不另外手抄宣告。`ReadableBuffer` 只存在於 stub 中、供型別檢查器使用：它代表任何支援 buffer protocol 的物件（例如 `bytes`、`bytearray` 或 `memoryview`），執行期無法從 `fastbase91` 匯入。使用範例見[使用方式](usage.md)。

```python
--8<-- "bindings/python/python/fastbase91/__init__.pyi"
```

### 例外

| 例外 | 發生時機 |
| --- | --- |
| `DecodeError` | 只有在嚴格模式下，解碼遇到 basE91 字母表以外的位元組時發生。它是 `ValueError` 的子類別；`offset` 與 `byte` 屬性會指出該無效位元組。對一次性 `decode` 而言，`offset` 是輸入內的位置；對 `Decoder(strict=True).update()` 而言，`offset` 包含先前更新已消耗的位元組，是整個串流中的位置。 |
| `TypeError` | `encode`、`decode` 或 `update()` 收到不支援 buffer protocol 的物件，例如 `str`。 |
| `BufferError` | buffer 不是連續的一維 buffer，或其中的元素不是單一有號或無號位元組。 |
| `ValueError` | `finish()` 關閉 `Encoder` 或 `Decoder` 後，再對該物件呼叫 `update()` 或 `finish()` 時發生 `ValueError`。 |
| `MemoryError` | 無法配置輸入或輸出 buffer，或 Rust core 回報 `AllocationFailed`。 |
| `RuntimeError` | Rust core 回報不應發生的內部錯誤；或者同一個 `Encoder` 或 `Decoder` 物件被兩條執行緒同時使用而發生借用衝突，後者可能拋出 `RuntimeError`。見[自由執行緒](free-threading.md)。 |

## Rust

完整的 Rust API 參考由原始碼產生，發佈於 [docs.rs](https://docs.rs/fastbase91-core)。

### 錯誤

| 錯誤型別或 variant | 回傳它的 API | 出錯時的狀態 |
| --- | --- | --- |
| `OutputTooSmall` | `encode_into`、`Encoder::update` | 所需大小可由 `required()` 取得。容量會先檢查，因此狀態與輸出都不變。 |
| `EncodeError::AllocationFailed` | 啟用 `alloc` 的 `encode()` | 由一次性配置路徑回報。 |
| `DecodeError::OutputTooSmall` | `decode_into`、`Decoder::update` | 容量會先檢查，因此狀態與輸出都不變。 |
| `DecodeError::InvalidByte { byte, offset }` | 嚴格模式下的 `decode`（需要 `alloc`）、`decode_into`、`Decoder::update` | 解碼器狀態不變；失敗呼叫寫入的輸出必須丟棄。對 `Decoder::update` 而言，`offset` 是該區塊內的位置；對一次性 `decode` 與 `decode_into` 而言，是完整輸入內的位置。 |
| `DecodeError::AllocationFailed` | 啟用 `alloc` 的 `decode()` | 由一次性配置路徑回報。 |

一次性的 `encode()`、`decode()` 函式與兩個 `AllocationFailed` variant 都只在啟用 `alloc` feature 時存在；預設的 `std` feature 會啟用它。`encode()` 回傳 `EncodeError`；由於它事先依 `max_encoded_len` 的上界預留足夠容量，實務上只會遇到 `AllocationFailed`。`Encoder::finish` 回傳 `([u8; 2], usize)`，不會失敗。`Decoder::finish` 回傳 `Result<Option<u8>, DecodeError>`。
