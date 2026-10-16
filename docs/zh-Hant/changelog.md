# 變更紀錄

每個版本都產生位元組相同、可互通的標準 basE91；升級絕不會改變編碼輸出。Python 套件 `fastbase91` 與 Rust crate `fastbase91-core` 各自獨立編版號；除非該版另有註明，下列每一版都是兩個套件以標題所示的版號一起發布。

## 0.1.4

這一版只發布 Python 套件；`fastbase91-core` 維持 0.1.3 不變。

- **Linux ARM64 wheel**（glibc 2.28 以上）：涵蓋一般 CPython 3.11 以上，以及自由執行緒 3.14t 與 3.15t 以上，所以 Apple silicon 上的 Docker、AWS Graviton 等 ARM64 Linux 機器不必再從原始碼建置。
- **Alpine 等以 musl 為基礎的 Linux**（musl 1.2 以上）新增 x86_64 與 ARM64 的 wheel，供一般 CPython 3.11 以上使用。這些 wheel 內含 Rust 在 musl 上需要的 GCC `libgcc_s`，它的授權寫在 `THIRD-PARTY-LICENSES`，現在每個發行物都會附上這份檔案。
- 編碼與解碼都沒有改動，輸出也完全相同。完整的 wheel 清單見[安裝與相容性](installation.md)。

## 0.1.3

- **docstring**：每個公開的 Python 函式、類別與方法都有 docstring，內容與型別 stub 一致，所以 `help()` 與編輯器提示顯示的文字相同。
- **Rust API 文件**：docs.rs 上新增啟用 `alloc` 的 `encode()` 與 `decode()` 的說明，以及 crate 總覽。
- **套件頁面**：PyPI 與 crates.io 現在會顯示 README，並連到文件網站與原始碼 repository。
- wheel 改用 PyO3 0.29.3 建置。編碼與解碼都沒有改動，輸出也完全相同；`no_std`、MSRV 1.81 與 `#![forbid(unsafe_code)]` 都維持不變。

## 0.1.2

- **編碼**改寫成一次處理多個位元組（word-at-a-time）的熱迴圈，並以一張成對查表輔助。
- **寬鬆解碼**新增了 8-byte 區塊快速路徑，而且在落單符號被消化掉後會立刻重新啟用——所以帶換行、串流的輸入也吃得到加速。
- 輸出與先前版本完全相同；`no_std`、MSRV 1.81 與 `#![forbid(unsafe_code)]` 都維持不變。見[效能測試](benchmarks.md)。

## 0.1.1

- **編碼**熱迴圈改成直線式。
- **解碼**拆成寬鬆（預設）與嚴格兩條路徑，加速了預設路徑。

## 0.1.0

- 首次發佈：Python 套件 `fastbase91` 與 `no_std` Rust crate `fastbase91-core`。
