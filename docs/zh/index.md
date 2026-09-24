# 總覽

[English](../en/index.md)

fastbase91 透過安全的 Rust core 與 Python 擴充套件提供標準 basE91 編解碼。basE91 使用 91 個可列印 ASCII 字元表示二進位資料，輸出大小約增加 23%，相較之下 base64 約增加 33%。

## 套件

| 套件 | 用途 |
| --- | --- |
| `fastbase91` | 以 PyO3 與 Rust core 實作的 Python 套件 |
| `fastbase91-core` | 支援 `no_std`、可重用的 Rust crate |

兩個套件使用相同的 codec。fastbase91、本儲存庫的純 Python 參考實作與 pybase91 會產生逐位元組相同的輸出，也能彼此解碼對方的輸出。

## 設計重點

- **可攜性：** Python wheel 涵蓋 Windows、macOS 與 Linux；Rust core 不使用 `std` 或 allocator 也能執行。
- **並行能力：** 大型 Python one-shot 呼叫會釋放 GIL，並發行專用的 free-threaded wheel。
- **安全性：** Rust core 使用 `#![forbid(unsafe_code)]`。
- **控制能力：** 兩種 API 都提供串流操作，解碼時也可拒絕 basE91 字母表以外的位元組。

fastbase91 並不以單執行緒最快為定位，而是兼顧 free-threading、可攜性、安全性、串流與 strict 解碼。實測取捨請見[方案比較](comparison.md)與[效能測試](benchmarks.md)。

## 快速開始

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
assert encoded == b"TPwJh>A"
assert fastbase91.decode(encoded) == b"hello"
```

完整的 Python 與 Rust API 請繼續閱讀[使用方式](usage.md)。

