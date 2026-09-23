# fastbase91 專案 Brief

> 狀態：2026-09-23，供新工作區正式實作使用。本文件是設計層級的決策與契約；精確公式、reference vectors 與微觀實作規則於實作階段對 Henke C reference 逐一釘定。

## 一句話目標

建立一個完整、可長期維護、標準相容的 basE91 實作：底層是獨立且通用的 Rust core，上層是以 PyO3／maturin 建置的 Python library；第一版優先追求正確性、零 `unsafe`、乾淨的 `no_std` core、真正的 free-threading（CPython 3.14t 與 3.15），以及比現有套件更完整的跨平台 wheels。

## 使用者在意的原則

- 必須是 Joachim Henke 的標準 basE91，輸出需與原始 C reference 完全相容。
- 技術品質比商業價值、行銷或「全部自己寫」更重要。
- 若別人的 Rust core 寫得好且持續維護，可以直接採用；不排斥薄 wrapper。
- 願意投入少量工作換取明顯的安全性、速度、可維護性或更多可用環境。
- 不願投入大量工作，只換得自己用不到的邊際功能。
- 初期希望朝自己維護的 Rust 程式碼零 `unsafe` 前進。
- `no_std` 若能以低成本帶來乾淨架構與更多環境，值得支援；它不應拖慢 Python library 的主要進度。
- Python 端需要比現有 `pybase91` 更完整，尤其是 CPython 3.14／3.15、3.14t、Windows、較廣的 Linux 相容性與真正的 free-threading 設計。
- 信任成熟的打包工具（maturin、auditwheel）：其產出的 wheel tag 與相容性由工具負責，不自行防禦；工具的 bug 由上游修，不列入本專案的維護面。
- 套件名稱不能撞到現有 PyPI project。

## 目前的方向與決策

### 已決定

1. 實作標準 Henke basE91，不採用 `base91x` 或其他固定 13-bit 非標準格式；以 Henke C reference 為唯一權威。
2. 建立自己的、專注於標準 basE91 的 Rust core；現有實作作為 reference、benchmark 與設計資料。單一 monorepo，core 與 Python binding 為兩個 Cargo package。
3. core 與 Python binding crate 都使用 `#![forbid(unsafe_code)]`。
4. core 從第一天保持 `no_std`，核心 state machine 不依賴 heap，且對任何輸入都不 panic。
5. 用 caller-provided slice 作為最底層 API。
6. core 從第一版採 stateful `Encoder`／`Decoder` 與 `update`／`finish`，讓 one-shot 與 streaming 共用同一套邏輯。
7. 第一版 streaming 要求「每個 input chunk 都有足夠大的 output buffer」，容量以對所有 state 成立的上界判定；不做 output 滿時的 partial-consumption／resume。
8. Python API 為 `bytes -> bytes`（編碼輸出為 ASCII bytes）；需要 `str` 的呼叫者自行 `.decode("ascii")`／`.encode("ascii")`。第一版做好 one-shot 與 streaming（core 已 stateful，成本低）。
9. decoder 以 options struct 設定兩個正交開關（是否拒絕非 alphabet、是否要求 canonical），標 `#[non_exhaustive]`；v1 實作「拒絕非 alphabet」，canonical 保留擴充、不在 v1 提供。
10. 最低支援 CPython 3.11（`abi3-py311`）；free-threaded 3.14t 用版本專屬 wheel；3.15+ 用 PEP 803 stable ABI（abi3t）。三種 wheel 的 tag 由 maturin 產出並信任之。
11. license 採 `MIT OR Apache-2.0`。
12. 若未來真的需要 `unsafe` fast path，必須由 profiling 與實際 workload 證明，而且要隔離、可選、可 differential-test。

### 延後處理

- `std::io::Read`／`Write` adapters。
- async／Tokio adapters。
- output buffer 太小時只消耗部分 input 的複雜 resume API。
- 單一 basE91 stream 的平行化。
- 非標準 SIMD wire format。
- canonical／完整性驗證（保留擴充空間，v1 不做）。

## 現有生態調查

### `base91x 1.0.1`

- PyPI wheel 是 `py3-none-any`、`Root-Is-Purelib: true`，實際執行純 Python，沒有載入 repo 裡的 C／C++。
- 採固定 13-bit 與不同 alphabet，官方也明確說明不相容標準 basE91。
- 主要價值是避免 `"`、`'`、`\`，方便嵌入 JSON 與 C/C++ literals。
- 不適合作為本專案的標準格式核心。

### Python `base91 1.0.1`

- 2016 年發布的純 Python 標準 basE91。
- 套件名稱 `base91` 已被占用。

### `pybase91 0.2.2`

- 2026-03-30 發布，正是 Rust + PyO3 + maturin 的標準 basE91 extension。
- Python API 為 `bytes -> bytes`，提供 one-shot 與會把全部輸出累積到 `finish()` 的 streaming class。
- 實際輸出已與現有 helper 在 0 B 到 1 MiB 測試資料逐 byte 相符。
- 現有 wheels：CPython 3.11–3.13、macOS x86_64/arm64、Linux x86_64/aarch64。
- 缺少 CPython 3.14、3.14t、Windows、`abi3`，Linux wheel 僅到 `manylinux_2_36`。
- binding 只接受 `bytes`，未支援完整 buffer protocol，沒有釋放 GIL，也沒有宣告完整 free-threaded 語意。
- PyPI 名稱 `pybase91` 已被占用。

### `base91-rs 0.2.2`

- `pybase91` 與 `base91-rs` 是同一作者 Frederic Ruget（GitHub `@douzebis`）、同一 monorepo 的兩個發布物。
- `base91-rs` 是通用 Rust core；啟用它的 optional `python` Cargo feature 後，由 maturin 建出 `pybase91`。
- Rust core 的效能分析與最佳化工作相當深入，有 one-shot、streaming、I/O adapter、C ABI、C reference vectors 及非標準 SIMD 格式。
- 預設測試在 Apple Silicon／Rust 1.98 通過：39 unit tests、9 reference-vector tests、11 doctests。
- 主要開發集中在 2026-03-28 至 2026-03-31，之後約半年沒有 commit；目前還不足以判斷長期維護狀況。
- crates.io 約 194 次下載、GitHub 1 star，尚屬非常年輕的專案。

已確認的技術缺口：

- 宣稱 `no_std-compatible`，但 v0.2.2 執行 `cargo check --no-default-features` 會因 `simd`、`Vec` 與 `std::*` 引用而失敗。
- 文件宣稱 CI 測 MSRV 1.74，實際 `Cargo.toml` 是 `rust-version = "1.91"`，workflow 也沒有 MSRV job。
- 高效能 safe public API 內部使用 `spare_capacity_mut()`、raw writes 與 `set_len()`；unsafe 範圍有註解，但 repository 沒有實際 cargo-fuzz、Miri 或 sanitizer workflow。
- size hint 使用未檢查的 `input_len * 16`、`input_len * 7`，在 32-bit 或理論極大輸入下值得改成 checked arithmetic。
- 只有忽略非 alphabet byte 的 lenient decode，缺少 strict decode API。
- 標準 core、非標準 SIMD、C ABI、I/O 與 Python binding 集中在同一 crate，feature boundary 不夠乾淨。

### 獨立 Rust crate `base91 0.1.0`

- 作者 `dnsl48`，純通用 Rust crate，與 Python 無關。
- 完全沒有 `unsafe`，支援 `no_std`、iterator、canonical 與 XML-friendly alphabet。
- crates.io 約 13.8 萬下載。
- 最後實質程式更新在 2021 年；2022 年後沒有功能開發。
- 效能顯著落後 `base91-rs`，因此可作安全 API 參考，但不是高速核心首選。

目前沒有找到第三個更成熟、持續維護且效能優於 `base91-rs` 的標準 basE91 Rust crate。

## 專案定位與 v1 成功條件

本專案的差異化不在原始編碼速度。兩個 Rust extension 的 one-shot 吞吐量預期同級，`pybase91` 也已與 helper 逐 byte 相符。值得投入的理由是：`no_std`、自行維護且零 `unsafe` 的乾淨 core、明確的 streaming 與錯誤契約，以及比現有套件更完整、更可靠的 wheel 覆蓋與 free-threading 設計。

要誠實面對的是：對 Python 使用者而言，v1 相對 `pybase91` 可見的差異，多數（wheel 覆蓋、buffer protocol、`detach`、free-threading 宣告）是既有作者短時間內就能補齊的；唯一較難補、且真正功能性的差異，是「逐 chunk 立即回傳」的 streaming。因此把 streaming 做成一級功能、把契約與打包做紮實，才是這個定位站得住的關鍵。

目標 workload 是**通用 library**：payload 不分大小都要支援，因此設計同時顧小 payload 的 latency 與大 payload 的 throughput，`detach` 不預設每次都做。v1 成功條件：

1. 與 Henke C reference 逐 byte 相容（C reference 為唯一權威；若與任何 helper 分歧，登記為 helper 的 bug，不改變本實作）。
2. 明確且經測試的 buffer／複製契約，含 free-threading 下的安全性與並行前提。
3. streaming 為真正的 bounded-memory：輸出立即回傳、不累積到 `finish`，記憶體隨 chunk 而非總訊息長度成長，且失敗與結束行為符合契約。
4. 可靠的 `abi3`（3.11+）、3.14t 與 3.15 wheels，通過逐直譯器的安裝與相容性測試。
5. 在通用 workload（小 payload latency 與大 payload throughput 皆然）上達到效能目標。

延後到有需求才做：通用 buffer 的非連續／多維特例、內建 batch 平行化、更廣的平台矩陣、canonical 驗證、`unsafe` fast path。

## 已測得的效能基線

測試環境為 Apple M1。不同執行時的系統負載會影響絕對數字，應重視數量級與相對差異；這些數字作為該環境的驗收值，不作為所有 CI runner 的硬 gate。

### Python 實作

1 MiB 隨機資料：

| 實作 | Encode | Decode |
|---|---:|---:|
| `pybase91` Rust extension | 約 0.98 ms | 約 1.17 ms |
| 現有純 Python helper | 約 136 ms | 約 205 ms |

即使加入 ASCII 轉換，Rust extension 仍快約 130–175 倍。這個倍率是「native 相對純 Python」的差距，不是 fastbase91 相對現有 `pybase91` extension 的增量收益；本專案不以超越既有 extension 的原始速度為目標。

### Rust crates（Criterion）

1 MiB：

| 實作 | Encode | Decode |
|---|---:|---:|
| `base91-rs` safe public API | 約 1.03 GiB/s | 約 986 MiB/s |
| `base91-rs` unchecked API | 約 1.19 GiB/s | 約 1.30 GiB/s |
| `base91 0.1.0` | 約 451 MiB/s | 約 725 MiB/s |

### 零 `unsafe` proof of concept

用直接迴圈、預先配置並以安全索引寫入：

| 實作 | Encode | Decode |
|---|---:|---:|
| `base91-rs` 內部 unsafe | 約 1,070 MiB/s | 約 888 MiB/s |
| 完全 safe、預配置後索引 | 約 1,081 MiB/s | 約 693 MiB/s |

這證明零 `unsafe` 並不必然造成大幅效能損失：encode 可持平，簡單 safe decode 吞吐量約低 22%，仍接近 700 MiB/s。此 PoC 尚未計入容量 preflight、strict 分支、buffer 快照與 Python 配置的成本，最佳化優先序須以固定 benchmark protocol 對完整路徑重新確認。

## 建議 repository 架構

```text
fastbase91/
├── Cargo.toml                 # workspace
├── crates/
│   └── fastbase91-core/
│       ├── src/{lib,encode,decode,tables}.rs
│       ├── tests/{reference_vectors,differential,streaming}.rs
│       └── benches/throughput.rs
├── bindings/
│   └── python/
│       ├── Cargo.toml
│       ├── pyproject.toml
│       ├── src/lib.rs
│       ├── python/fastbase91/{__init__.py,__init__.pyi,py.typed}
│       └── tests/
├── fuzz/
└── .github/workflows/
```

將 PyO3 與 core 分成不同 Cargo packages。Core 不應出現 PyO3 dependency 或 `python` feature；Python crate 單向依賴 core，並以 workspace path dependency 帶著 core，兩者版號獨立，不需 lockstep、不需 core 上 crates.io。

## Rust core 設計

### Feature layers 與安全契約

```toml
[features]
default = ["std"]
alloc = []
std = ["alloc"]
```

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;
```

| 層級 | 能力 |
|---|---|
| core only | state machine、caller-provided slices、無 heap |
| `alloc` | `Vec<u8>` one-shot convenience API |
| `std` | 未來可加入 `Read`／`Write` adapters |

安全契約：

- `#![forbid(unsafe_code)]` 只約束本 crate，不涵蓋 PyO3、allocator 與 CPython。binding crate 同樣 `forbid(unsafe_code)`——PyO3 支援此設定，binding 需要的 API 皆為 safe，使「自己維護的 Rust 零 unsafe」成為編譯器守住的事實。未來若引入 unsafe，需調整 crate 邊界或 lint 政策，不能僅以局部 `allow` 覆蓋。
- **core 對任何輸入都不 panic。** `forbid(unsafe_code)` 把失敗模式從 UB 換成 panic（索引越界、位移溢位），而 panic 穿過 PyO3 會變成繼承 `BaseException` 的 `PanicException`，Python 端 `except Exception` 接不到。「不 panic」是明確契約，由 fuzz 目標以此為斷言。
- **配置失敗以 `Result` 回報，不 panic。** `alloc` 層使用 fallible allocation（`try_reserve_exact` 一類），避免 `Vec::with_capacity` 在 OOM 時 abort／panic；如此「任何輸入、任何配置結果都以 `Err` 回報」在 slice 層與 alloc 層一致，Python 的 `MemoryError` 對映自然落出。

### 容量與整數邊界

`max_encoded_len`／`max_decoded_len` 各回傳一個**對所有 state 成立、且已含 `finish` 尾端**的長度上界；`update` 在任何 state 下所需容量都不超過對應上界，因此**不對外暴露 state-aware 容量查詢**，固定 chunk 的使用者能在編譯期算出 buffer 大小。精確公式與推導留到實作，並對 reference 驗證。設計層面：

- 使用 checked 算術，並以商／餘數分解避免中間值過早 overflow。
- 區分「數學上界」與「可配置容量」：`usize` 表示得下不等於配置得出來（Rust `Vec` 受 `isize::MAX`、Python 用 signed `Py_ssize_t`）；配置層另檢查自己的限制。

### 錯誤型別

core 的錯誤是不配置記憶體的結構化 enum，攜帶 invalid byte、byte offset、required capacity，並實作 `core::fmt::Display`；MSRV 若在 1.81 以上，直接實作 `core::error::Error`。所有公開錯誤 enum（`DecodeError`、`EncodeError` 等）**從 v1 就標 `#[non_exhaustive]`**：日後新增 canonical 錯誤種類，對下游的 exhaustive `match` 才不是破壞性變更（`#[non_exhaustive]` 只能在 1.0 前加，事後補本身就是 breaking）。

### Stateful core API

- `update` 遇 `OutputTooSmall` 時 state 與 output 都不變；成敗只取決於 `(state, input.len(), output.len())`（preflight、以上界判定），可預測、零回滾。
- `finish` 消耗 `self`，回傳固定大小尾端，永不因容量失敗、不配置 heap：encoder 尾端最多兩個符號。
- decoder 的 `finish` 回傳 `Result<Option<u8>, DecodeError>`：v1 永不從 `finish` 回錯，但保留錯誤通道，讓日後 canonical 驗證可加入而不改簽章。

```rust
pub struct Encoder { /* queue, nbits */ }

impl Encoder {
    pub const fn new() -> Self;
    // 任何 state 下所需容量 ≤ max_encoded_len(input.len())；OutputTooSmall 時 state/output 不變。
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<usize, OutputTooSmall>;
    #[must_use]
    pub fn finish(self) -> ([u8; 2], usize);
}

// #[non_exhaustive] 讓日後加軸（如 canonical）不破壞相容；
// 它也擋外部 struct literal，故 v1 提供 const fn 建構子與 Default。
#[non_exhaustive]
pub struct DecodeOptions {
    pub reject_non_alphabet: bool, // false = lenient（忽略非 alphabet）
}

impl DecodeOptions {
    pub const fn new() -> Self; // reject_non_alphabet = false
}

pub struct Decoder { /* queue, nbits, pending, options */ }

impl Decoder {
    pub const fn new(options: DecodeOptions) -> Self;
    // 容量以對所有 state 成立的上界判定；OutputTooSmall 時 state/output 不變。
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError>;
    pub fn finish(self) -> Result<Option<u8>, DecodeError>;
}
```

`reject_non_alphabet`：`false` 忽略所有非 alphabet bytes（含 whitespace、NUL、高位元），與 C reference 相容；`true` 遇非 alphabet byte 回傳含 byte offset 的錯誤。

canonical 驗證是另一條正交的軸（「忽略換行但要求 canonical」，如折行包裝的 base91，是合理組合），因此採 struct 而非扁平 enum。**v1 不宣告 `require_canonical` 欄位**——「保留擴充」由 `#[non_exhaustive]` 屬性達成，日後實作 canonical 時新增為第二個欄位即可；不放一個可設 `true` 卻無效的 dead 欄位（那會在 canonical 上線時，讓既有設 `true` 的呼叫端突然開始拒絕輸入，是不可回頭的契約破壞）。canonical 違規可能發生在符號對中途（encoder 不會產生的 pair 值），不只尾端，因此 `update` 與 `finish` 都需能回報，現有簽章已足夠。

錯誤交易性：容量 preflight 失敗時 state 與 output 都不變；字元錯誤時 state 不變，但該次已寫入 caller slice 的 output 內容未定義，呼叫者須整段重送、不可沿用。offset 為 chunk-local（呼叫者需整條 stream 位置時自行加總）。

### basE91 相容性規則

正確性以 Henke C reference 為唯一權威；實作時對照 reference 原始碼與一組固定版本＋hash 的 reference vectors 逐一驗證。設計層面必須守住：

- 標準 91 字元 alphabet；`=` 與 `"` 都是一般資料符號，沒有 Base64 式 padding。
- lenient 忽略所有 alphabet 以外的 bytes。
- **相容性難點在 decode 的尾端 bit 處理**：解碼在符號對之間依剩餘 bit 數走不同分支，收尾時未輸出的 bits 要正確補成最後一個 byte。這是與 reference 對不齊的常見來源，必須由涵蓋各類尾端 state 的 vectors 釘住。
- 具名 smoke vector：`b"hello" -> b"TPwJh>A"`。

### One-shot 與 slice API

只在 `alloc` feature 下提供配置版；也提供不需 `alloc` 的 `encode_into`／`decode_into` slice API。兩者內部都呼叫相同的 `Encoder`／`Decoder`，不維護第二套演算法。

```rust
#[cfg(feature = "alloc")]
pub fn encode(input: &[u8]) -> Result<Vec<u8>, EncodeError>;

#[cfg(feature = "alloc")]
pub fn decode(input: &[u8], options: DecodeOptions) -> Result<Vec<u8>, DecodeError>;
```

### Streaming 範圍與訊息邊界

第一版核心支援真正 bounded-memory streaming：呼叫者反覆提供 input chunk 與足夠大的 output slice，立即取走輸出，最後 `finish()` 取回固定尾端。

**framing 由呼叫者負責。** `finish` 代表整條訊息結束，不是一般 flush；獨立編碼的輸出不能直接串接成同一條 stream（串接會解出與原意不同的 bytes，且 lenient 會忽略分隔字元，插入換行也無法分界）。文件與 API 須明說此點。

不在第一版支援：output 滿時只消耗部分 input；resume protocol；async；同一 stream 的多執行緒 update。不同 instance 可由不同執行緒同時使用；同一 instance 不承諾 concurrent calls。

## Python API 設計

套件名稱暫定 `fastbase91`。native module 放在 `_fastbase91`；Python facade 只放 typing 與文件，穩定 API 以 `from ._fastbase91 import ...` 直接 re-export，不額外包一層 Python function（避免每次呼叫多一個 frame，影響小 payload latency）。

```python
def encode(data: ReadableBuffer, /) -> bytes: ...
def decode(data: ReadableBuffer, /, *, strict: bool = False) -> bytes: ...

class Encoder:
    def __init__(self) -> None: ...
    def update(self, data: ReadableBuffer, /) -> bytes: ...
    def finish(self) -> bytes: ...

class Decoder:
    def __init__(self, *, strict: bool = False) -> None: ...
    def update(self, data: ReadableBuffer, /) -> bytes: ...
    def finish(self) -> bytes: ...
```

### 型別與錯誤契約

- API 一律 `bytes -> bytes`（編碼輸出為 ASCII bytes）。需要 `str` 的呼叫者自行轉換；不接受 `str` 輸入，藉此避開 ASCII 驗證、offset 單位與 surrogate 的整套邊界問題。
- 空輸入回傳空 `bytes`。
- 型別錯誤的輸入拋 `TypeError`。
- `strict=True` 對應 core 的 `reject_non_alphabet`，精確定義為「拒絕非 alphabet byte」，不是 canonical 驗證；未來 canonical 走加法式的 `canonical: bool`，不改變 `strict=True` 既有行為。
- 解碼錯誤拋 `ValueError` 的子類別（類比 `binascii.Error`）；`offset` 放在 base 子類，`byte` 只放在「非 alphabet byte」錯誤上——如此日後 canonical 在 `finish` 失敗（無對應 byte）時可沿用同一 base、不必更動 `byte` 的型別契約。屬性名一旦公開即契約。

### Streaming class 契約

- 建構時給定選項（decoder 的 `strict` 在 `__init__`，不在每次 `update`）。
- `update(chunk)` 立即回傳該 chunk 產生的輸出，不累積到 `finish`；`finish()` 回傳尾端，且**不論成功或拋錯都關閉 instance**（對映 Rust `finish(self)` 消耗 self；未來 canonical 會讓 `finish` 可能拋錯，此語意須從 v1 定死），之後再呼叫 `update`／`finish` 一律拋例外——與 framing 契約呼應，讓「串接兩段獨立編碼」這種 lenient 下看不見的誤用響亮失敗。
- `update` 回傳的是**已解碼前綴**，不代表整條訊息已通過驗證；後續錯誤不能撤回先前交付的 bytes（strict 遇後段非法字元時即如此），此語意從 v1 明說。
- 交易邊界涵蓋整個公開方法：core 已推進 state 但建立 Python 物件失敗時，須維持「呼叫者沒收到輸出就等於沒消耗」，不得重送時重複消耗。
- 錯誤 offset 在 Python 層回報整條 stream 的累計位置（core 內部 chunk-local，Python 物件加總成本近零）。
- 同一 instance 的並行 `update` 以 PyO3 pyclass 的 borrow 檢查響亮失敗（`RuntimeError`），恰好就是「不承諾 concurrent update」的語意；每個 stream／thread 用自己的 instance，不保證跨執行緒的訊息順序。

### Buffer 與 free-threading 契約

安全性建立在明確前提上，不承諾「取得一致快照」：

- `bytes`（不可變）：保留物件生命週期後走可 detach 的借用路徑，可安全共享給多個並行呼叫。
- 其他所有 buffer（含 `bytearray`、`memoryview`）：**會複製，但呼叫期間來源不得被並行修改**（呼叫者的前提）。在 Rust 裡對「正在被別的執行緒改寫的記憶體」形成 `&[u8]` 讀取即 data race＝UB，因此複製要嘛走 CPython C API（Rust 全程不持有 aliasing slice），要嘛以此前提為準；不能只憑 buffer export 或鎖住某個 view 就宣稱安全。需要共享可變資料時，由呼叫者同步所有相關讀寫。
- buffer 接受範圍 v1 取嚴：只接受連續、單 byte 格式的 bytes-like（`bytes`、`bytearray`、連續 `memoryview`），非連續與多維一律以明確例外拒絕。理由是放寬（日後接受非連續／多維並定義 flatten 與 byte-order）是加法、收緊是破壞，故 1.0 前先窄。

### 執行緒與並行

- module 明確宣告 free-threading support：`#[pymodule(gil_used = false)]`（宣告，非安全性證明）。
- one-shot calls 不共享 state，可在 3.14t／3.15t 下真正平行。
- 夠大的 one-shot Rust 計算在 `Python::detach()` 中執行；小 payload 的 detach／reattach 成本應量測，不預設每次呼叫都 detach。
- sub-interpreter 不支援（PyO3 module 在 `concurrent.interpreters` 中 import 會失敗），與 free-threading 一併於文件明寫。

### 記憶體行為

bounded-memory core 不代表 Python one-shot 也 bounded-memory：輸入複製、Rust output、Python `bytes` 可能使 peak RSS 達輸入的數倍；`abi3` 下輸出長度事前未知且 `_PyBytes_Resize` 不在 limited API，故有一次 `Vec(上限) -> bytes(精確)` 的複製。應量測 peak RSS，並定義過大輸入與配置失敗時的行為。binding 自己的兩次配置（輸入複製、輸出 `Vec -> bytes`）也走 fallible 路徑（`try_reserve_exact`、`PyBytes::new_with` 等），維持「配置失敗回 `MemoryError`」的契約，避免 PyO3 預設路徑 abort／panic（後者正是穿過 `except Exception` 的 `PanicException`）。

## Python ABI 與 wheel 策略

建議採 PyO3 0.29+、maturin 1.15+，並用 maturin-action 建置。

### ABI

| Python | wheel |
|---|---|
| 一般 CPython 3.11+（含 3.15、3.16… GIL） | `abi3-py311`，每平台一份，forward-compatible |
| CPython 3.14t | 版本專用 `cp314-cp314t` wheel |
| CPython 3.15+（一般與 free-threaded） | PEP 803 stable ABI（`abi3t`），單一 wheel 涵蓋 3.15+ |

最低版本定在 3.11：PyO3 的 buffer protocol 在 limited API 下自 CPython 3.11 起才可用，而 buffer protocol 是本 library 的核心路徑。3.14t 沒有 free-threaded stable ABI，需版本專屬 wheel。PEP 803 已 Final、目標 3.15。三種 wheel 的 tag 與相容性由 maturin／auditwheel 產生並信任之；本專案不自行驗證或防禦 tag 正確性，工具的 bug 由上游修。

建置面的事實（非測試負擔）：一次 maturin build 只產一種 stable-ABI family，因此每平台以不同直譯器分別建置；一組 cargo feature 同時開 abi3 與 abi3t，由建置用的直譯器決定產物。

**支援邊界（明文宣告，不補救）**：free-threaded 3.15+ 需要夠新、認得 `abi3t` tag 的安裝器；使用過舊或第三方安裝器時可能退回 sdist、需要 Rust toolchain。此為使用者環境問題，錯誤訊息已清楚指出缺編譯工具，本專案不為此另發 fallback wheel 或做相容處理。Windows 的一般 3.14 與 3.14t 放在分開 CI jobs。

### 初期平台矩陣

- Linux manylinux（起始 image 與升級節奏另評估，第一個可預期的淘汰是 manylinux2014 → `manylinux_2_28`）：x86_64、aarch64。
- Linux musllinux：x86_64、aarch64，若增加的 CI 成本可接受。
- macOS：x86_64、arm64。
- Windows：x86_64；arm64 視 GitHub runner 與工具鏈穩定度加入。
- 一般 CPython（含 3.15）與 free-threaded（3.14t／3.15t）都做安裝後 smoke／compatibility tests（在對應直譯器上裝該 wheel、import、跑核心測試）。
- 發布 sdist，並驗證從 sdist 建置。

### 發布與供應鏈

- 預期 artifact 清單由支援政策／build matrix 生成，不從當次探索到的 interpreter 反推（interpreter 消失時「預期」不應一起縮水）；支援矩陣的縮減是人工產品決策。
- build job 輸出綁定版本、source commit、檔名與 sha256 的 manifest；test 與 publish job 都比對它，確保「產物完整且為同一批」。
- 安裝測試只用 wheel；另從獨立解壓的 sdist 建置，Rust crate 以 `cargo package` 從打包後的 crate 驗證建置、授權與 feature。
- **PyPI 上傳不是原子交易**：發布後做遠端清單／hash 核對與只補缺檔的流程；同一版本不能換檔，只能 yank + post-release（寫入 runbook）。本地 gate 綠燈不代表遠端完整。
- 明訂 `Requires-Python`、MSRV 與最低 OS 政策；`abi3` 不保證 glibc、CPU 指令集或 macOS deployment target 相容。發布採 PyPI 與 crates.io 的 Trusted Publishing (OIDC)，`id-token: write` 只給 publish job（綁 protected environment），trust binding 含 workflow 檔名（改名即失效，寫入 runbook）。
- 若納入 Henke C reference 或移植任何既有程式碼，保留其授權聲明。

## Correctness 與安全測試

以可驗收的有限集合與狀態覆蓋為主，reference 為獨立 oracle：

1. Henke C reference vectors（固定版本與 hash），逐 byte 驗證；不以持續最佳化的 fork `main` 當永久基準。
2. differential testing 對象包含**任意 encoded input**，不只 fuzz 自己 encoder 產出的合法資料。
3. 涵蓋各類尾端 state 的具名 vectors——單純列舉 8,281 個獨立 symbol pairs 不足以抓到尾端分支錯誤（從空 state 解一對時，錯誤條件會被「無 pending 就不補輸出」掩蓋），必須加上後續符號或不同前置 state，並列出各類尾端各至少一個 vector。
4. 全部 0、全部 1、0..255、固定 seed 隨機資料。
5. streaming：任意 chunk boundary、空 chunk、單字元 chunk、容量剛好（以上界定義）、容量少一 byte、錯誤後重試；結果等於 one-shot。
6. round-trip 與「one-shot == streaming」不能作為唯一 oracle（互相抵消的錯誤會過關），必須另有 reference 比對。
7. alphabet 與 decode table：鎖定完整、依序的 91-byte alphabet fixture，並獨立驗證 decode table，不只測兩表互為反函數。
8. strict 的 invalid byte 與 offset；strict 錯誤時 state 不變、output 視為未定義。
9. `OutputTooSmall` 時 state 與 output 不變。
10. 容量與中間值的 overflow boundary，含 32-bit（用合成長度與較寬整數 oracle，不需真的配置數 GiB）。
11. cargo-fuzz：encode、decode、round-trip、cross-implementation，並以「core 不 panic」為明確斷言。
12. Python free-threading：`PYTHON_GIL=0`／`-X gil=0` 的多執行緒壓力測試（正確性）；另以**未設強制旗標**的 free-threaded subprocess，檢查 import 前後 GIL 都維持關閉。

（Miri 對零相依、`forbid(unsafe_code)` 的 crate 幾乎沒有可抓的 UB，v1 不納入 CI；recipe 留在 `MAINTENANCE.md`，待 unsafe fast path 出現再啟用。）

## CI 要求

Rust：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test -p fastbase91-core --no-default-features
cargo test -p fastbase91-core --no-default-features --features alloc
cargo check --target thumbv7em-none-eabihf -p fastbase91-core --no-default-features
```

`no_std` 檢查針對 core package 單獨跑，避免 workspace feature unification 把 `std` 帶回來；`no-default-features` 與 `alloc` 組合要實際跑測試，不只 `check`。另外：core 與 binding 分別承諾 MSRV（core 的 MSRV 是對外承諾、可低；binding 的 MSRV 跟 PyO3 走，不是承諾）；MSRV 與 stable 都測；fuzz scheduled job（含不 panic 斷言）；benchmark 保留可比較基線、重要改動時跑。

Python：

- 一般 CPython 支援範圍（含 3.15）與 free-threaded（3.14t／3.15t）。
- `sys._is_gil_enabled()`／等效檢查，確認 import extension 不會重新啟用 GIL。
- ThreadPoolExecutor 對獨立 payload 的**正確性**進 gate；**scaling** 進 benchmark（共享 runner 上會 flaky，不當 gate）。
- 每個 wheel artifact 在目標 OS／直譯器實際安裝、import、跑測試。
- 與 Rust core／reference vectors 共用 fixtures。

## 長期維護策略與 CI 自動化

因為 basE91 是凍結標準，v1 之後的變動集中在「打包邊界」，本專案在同類中屬偏輕的維護負擔，但不是零：格式凍結不會消除容量、錯誤語意、編譯器與效能回歸的維護。以下為規劃預期，非量測值。核心原則是信任成熟工具、宣告支援邊界，不為下游環境或工具 bug 做防禦性投資。

### 可自動化的部分

- 排程跑 CI；matrix 加新 Python、丟 EOL Python。
- 相依套件版本 bump：由 Renovate 自動開 PR，符合範圍時綠燈自動合併。
- 發布用 Trusted Publishing (OIDC)，不保存長期發布憑證。
- 新 Python 的支援多半隨 maturin-action／PyO3 升版而來（跟著它們的發布節奏）。

### 需要人的部分

- **生態淘汰**：runner、manylinux image、GitHub Action 版本被移除或淘汰，造成「程式沒改、綠色管線卻變紅」，約每 6–12 個月一次。
- **相依升級是持續存在的風險，不是可預估的固定工時**：對既有 `abi3` wheel，新 Python 通常先做相容性測試即可；只有新建版本專屬 wheel、改用新 API 或修相容問題時才較可能需要升 PyO3。真的需要改 binding 時，Renovate 只能開 PR，實際修復是人工。
- **free-threading 是移動標靶**：3.14t、3.15 與後續 stable ABI 路徑仍在演進，須主動追蹤。cp314t 專屬 wheel 的壽命綁在 PyO3 支援窗上（PyO3 曾在 3.13 仍受支援時砍掉 3.13t）；停產政策——不為單一 free-threaded 版本釘舊 PyO3——寫入 `MAINTENANCE.md`。

### Renovate 策略

採用 Renovate（Mend 託管 App）。automerge 範圍不大（core 零相依；PyO3 走人工；主要是 dev-deps 與 Actions），不過度投資設定：

- automerge 範圍**按相依作用分類**，不只看 semver：逐 byte differential test 抓不到 MSRV 提升、wheel tag 改變、少包檔案、最低 OS 提升，故建置工具與發布相關 Action／workflow 不宜只因 patch／minor 就自動合併。
- **pre-1.0 相依（PyO3 為首）不依 semver 標籤自動合併**（`0.29 → 0.30` 是 minor 卻可能 breaking，一律人工）。
- GitHub Actions **只釘 commit SHA**（非可搬動 tag），由 Renovate 維護 digest。
- 限定排程時段；automerge 前提是 CI 為可信守門員。Renovate 只開 PR，不修壞掉的程式碼。

### 排程 workflow 與 60 天停用

GitHub Actions 可用 `schedule` 定期跑；public repo 連續 60 天無活動時排程 workflow 會被靜默停用。Renovate 活動能降低但不保證避開此規則。**獨立監看不能又是一個會因閒置而停用的排程**；最便宜的形式是 dead-man's switch：canary 成功時 ping 一個外部端點，端點在預期週期內沒收到即告警（該端點是另一個外部帳號，寫入 runbook）。

### Canary 與 release 的區別

每週 canary 是投報率最高的排程 job，但須說清楚更新哪些東西（最新 Rust 搭配鎖住的 `Cargo.lock` 不會測到最新相依）：一路是「已發布 wheel 在新 Python（含 pre-release）上執行」的相容性測試，一路是「更新相依後重建並實際安裝、執行」的測試。release 本身維持可重現的釘定版本。

### Runbook

`MAINTENANCE.md` 記錄：PyO3 升級步驟、free-threaded wheel 支援窗決策、wheel matrix 位置、release 觸發、Trusted Publishing 的 trust binding（含 workflow 檔名）、發布後補檔／yank 流程、dead-man's switch 端點。

## 效能目標與原則

第一版零 `unsafe` core 的合理目標（指定機器、編譯器與測試條件下的驗收值，非 CI 硬 gate）：

- 1 MiB encode：至少約 900 MiB/s，理想 1 GiB/s。
- 1 MiB decode：至少約 650–700 MiB/s。
- Python 1 MiB one-shot：encode／decode 各低於約 2 ms，含配置、輸出複製與 object conversion。
- 小 payload 另量 latency，不只看大 buffer throughput。

### Benchmark protocol

比較前固定量測條件：decode throughput 的分母（encoded 或 decoded bytes）、是否計入配置／清零／複製、相同 release flags、量測完整 Python 路徑（含 detach／reattach、輸出複製、object 建立），涵蓋隨機、全零、全 `0xff` 與實際資料。

### 最佳化順序

1. 直接、compiler-friendly 的 scalar loop。
2. 預先配置與安全索引。
3. 重排 branch／write layout（decode 的「吐 1 或 2 byte」資料相依是主要成本，可用固定寬度寫入配合足量 slack 化解，仍是安全 Rust）。
4. 視完整路徑量測結果，決定是否採用約 16 KiB 的 encode-pair lookup table（LLVM 常已把常數除法降成乘法／位移，且此表增加 cache 壓力與 `no_std` 空間成本）。
5. `encode_many()`／`decode_many()` 的多訊息 batch 平行化（延後；獨立 one-shot 加 detach 已足以讓呼叫端自行排程）。
6. 只有 safe 路徑確實不達需求時，才評估隔離的 unsafe writer。

不得為 benchmark 犧牲：標準 wire compatibility、overflow safety、invalid-input 定義、free-threading thread safety、可讀可 audit 的核心。

## `unsafe` 決策門檻

第一版保持 `#![forbid(unsafe_code)]`。只有同時符合以下條件才重新討論：

1. profiler 證明瓶頸是 safe buffer write，而非 object conversion、allocation 或其他 loop 結構。
2. 實際 workload 收益明顯，不是 synthetic benchmark 快幾個百分點。
3. unsafe 可隔離在很小的 private module。
4. 有完整 safety invariant 文件。
5. safe 與 unsafe 路徑可逐 byte differential fuzz。
6. 預設是否仍保持 safe 路徑，另行決定。
7. 因 `forbid` 涵蓋整個 crate，引入 unsafe 需調整 crate 邊界或 lint 政策，不能僅以局部 `allow` 覆蓋。

## 建議實作里程碑

### Milestone 0：Repository 與 CI 骨架

- Cargo workspace；`fastbase91-core` 與 Python binding crate（兩者 `forbid(unsafe_code)`）。
- formatter、clippy、test、MSRV、`no_std` checks；maturin 開發建置。
- **代表性煙霧測試**：一個帶 `&mut self` 方法的 `#[pyclass]`（對應 streaming），在三種 ABI tier（3.11 abi3、3.14t、3.15 abi3t）都能 build、install、import，且 import 後 GIL 維持關閉。提前確認 pyclass 在 abi3t 下可用、三段式打包成立，不等 core 最佳化完成。此步為三段式 ABI 的 go/no-go；若屆時 PyO3／maturin 的 abi3t 尚未端到端打通，退路是 3.15t 改用 `cp315-cp315t` 版本專屬 wheel（機制同 `cp314t`），其餘設計不動——先寫下退路，避免屆時重開整個 ABI 討論。

### Milestone 1：Safe scalar core

- alphabet／decode table；checked capacity（含商／餘數分解與 signed 上限）。
- 結構化錯誤型別；fallible allocation。
- `Encoder`／`Decoder` state machines，`finish` 回傳固定尾端（decoder 保留錯誤通道）。
- slice-based `update`／`finish`；`encode_into`／`decode_into`；`alloc` one-shot。
- lenient decode；「core 不 panic」契約與初步 fuzz。

### Milestone 2：Correctness hardening

- C reference vectors（固定版本與 hash）與各類尾端 state 的具名 vectors。
- differential tests，涵蓋任意 encoded input；alphabet／table 獨立驗證。
- arbitrary chunk-boundary property tests。
- strict（reject_non_alphabet）；fuzz（不 panic 斷言）、32-bit boundary checks。

### Milestone 3：Rust benchmarks 與 protocol

- 固定 benchmark protocol；小 payload latency；1 KiB／64 KiB／1 MiB throughput。
- 對比 `base91-rs`、`base91 0.1.0` 與 C reference。
- 只做基線；pair lookup table 與任何 unsafe 討論留到 Python 完整路徑量測之後（M4 之後）。

### Milestone 4：Python binding（one-shot 與 streaming）

- `encode`、`decode`（bytes↔bytes）；依 buffer protocol 契約分路。
- streaming `Encoder`／`Decoder` class（`update` 立即回傳、完整失敗／結束契約）。
- 大輸入 `Python::detach()`；`gil_used = false`；facade 直接 re-export；stubs、`py.typed`。
- Python differential 與 free-threading（含未強制旗標）測試；完整路徑初步量測（回饋 M3 的最佳化決策）。

### Milestone 5：Wheel coverage 與 release gate

- `abi3-py311`、`cp314-cp314t`、3.15+ 的 `abi3t`（每平台以不同直譯器分別建置）。
- Windows、macOS、manylinux；再評估 musllinux／Windows arm64。
- 逐直譯器 wheel install tests、sdist 與 `cargo package` 驗證、產物完整性與同批發布 gate、發布後遠端核對。

### Milestone 6：可選擴充

- Rust `std::io` adapters；batch parallel API。
- canonical 驗證（走既留的錯誤通道與 `require_canonical` 開關）。
- 只有經證明必要時才做 unsafe fast path。

## Naming 狀態

截至 2026-09-23，以下名稱的精確 package endpoint 都不存在：

- PyPI：`fastbase91`、`fast-base91`、`base91-native` 等候選名稱均為 404。
- crates.io：`fastbase91`、`fastbase91-core`、`fast-base91`、`fast-base91-core` 均為 404。

建議：PyPI distribution/import `fastbase91`；Rust public crate `fastbase91` 或 `fastbase91-core`；Python native module `fastbase91._fastbase91`。名稱尚未註冊，404 只代表查詢當下可用，並非保留。`fastbase91` 的「fast」是相對純 Python 而言，非相對既有 extension；這是有意識的取名，不應日後反過來推動 unsafe fast path。

應避免：`base91`（PyPI 與 crates.io 已占用）、`pybase91`（PyPI 已占用）、`base91-rs`（crates.io 已占用，易造成 ownership 混淆）。

## 重要 reference

- 標準 basE91 原始網站：[base91.sourceforge.net](https://base91.sourceforge.net/)
- 現有 `base91-rs`／`pybase91` monorepo：[douzebis/base91](https://github.com/douzebis/base91)
- `base91-rs` crate：[crates.io/crates/base91-rs](https://crates.io/crates/base91-rs)
- 舊的獨立 Rust crate：[dnsl48/base91](https://github.com/dnsl48/base91)
- `pybase91`：[pypi.org/project/pybase91](https://pypi.org/project/pybase91/)
- PyO3 free-threading guide：[Supporting Free-Threaded CPython](https://github.com/PyO3/pyo3/blob/main/guide/src/free-threading.md)
- PyO3 建置與 abi3 限制：[Building and Distribution](https://pyo3.rs/latest/building-and-distribution.html)
- PEP 803（free-threaded stable ABI）：[peps.python.org/pep-0803](https://peps.python.org/pep-0803/)
- maturin-action：[PyO3/maturin-action](https://github.com/PyO3/maturin-action)
- CPython extension free-threading HOWTO：[Python documentation](https://docs.python.org/3/howto/free-threading-extensions.html)

## 新工作區的第一個實作任務

先建立 workspace 與 `fastbase91-core`，完成下列最小垂直切片：

1. `#![no_std]` + `#![forbid(unsafe_code)]` 可編譯。
2. checked `max_encoded_len()`／`max_decoded_len()`（含商／餘數分解）。
3. stateful `Encoder::update()`／`finish()`，caller-provided slice、`finish` 回傳固定尾端。
4. `alloc` one-shot `encode()`。
5. `b"hello" -> b"TPwJh>A"` reference test。
6. 對 0..=256 bytes 的所有長度，與 Henke C reference 逐 byte 比對。
7. 任意 chunk boundary 的 streaming output 等於 one-shot。

完成這個切片後，再以相同結構加入 decoder（含 `reject_non_alphabet`，並涵蓋各類尾端 state 的 vectors）；先不要同時處理 Python binding、wheel matrix 或 unsafe 最佳化。
