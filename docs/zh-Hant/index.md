# 總覽

fastbase91 是給 Python 與 Rust 用的 basE91 編解碼器，為大家實際會卡住的地方而生：平行處理、跑在自由執行緒（free-threaded）Python 上、跑在 Windows 上，以及完全不靠作業系統就能跑。

basE91 是把二進位資料塞進 91 個可列印 ASCII 字元的一種做法。編碼後的文字大約比原始位元組大 23%，base64 則大約 33%——所以當你得把二進位資料塞進只能放文字的通道時，basE91 更省空間。

## 快速開始

```console
python -m pip install fastbase91
```

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
assert encoded == b"TPwJh>A"
assert fastbase91.decode(encoded) == b"hello"
```

Rust 使用者可執行 `cargo add fastbase91-core`；詳見[使用方式](usage.md)。

## 你會拿到什麼

| 套件 | 是什麼 |
| --- | --- |
| `fastbase91` | Python 套件（以 PyO3 與 Rust core 建置的編譯擴充） |
| `fastbase91-core` | 單獨的 Rust crate，支援 `no_std`，可跑在裸機環境 |

fastbase91 實作的是標準 [basE91](https://base91.sourceforge.net/)。它的輸出與原作者的參考實作逐位元組相同——core crate 的測試直接拿那份 C 程式碼來比對——所以能和任何標準 basE91 實作互相編解碼。想自己驗證的話：任何標準實作都會把 `hello` 編成 `TPwJh>A`，也就是上面快速開始裡的值。

## 它為什麼而生

- **並行處理。** 一次大型呼叫會把工作交給 Rust，並放掉 Python 的全域鎖（GIL），於是多個呼叫可以在多條執行緒上同時進行，而不是輪流排隊。見[效能測試](benchmarks.md)。
- **自由執行緒 Python。** 這個擴充被標記為在 no-GIL 版本下安全，所以匯入它不會偷偷把 GIL 又打開。[為什麼這很重要](free-threading.md)。
- **可攜性。** Python wheel 涵蓋 Windows、macOS 與 Linux；Rust core 不需要 `std`、甚至不需要記憶體配置器（allocator）也能跑。
- **安全性。** Rust core 是 `#![forbid(unsafe_code)]`——編譯器直接拒絕任何 unsafe 區塊。
- **控制力。** 兩種 API 都能串流（分段編解碼），而且解碼時可以拒絕 basE91 字母表以外的位元組，而不是默默略過。

接著看：[使用方式](usage.md)裡的完整 API。
