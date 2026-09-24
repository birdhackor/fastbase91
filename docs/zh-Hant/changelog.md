# 變更紀錄

每個版本都產生位元組相同、可互通的標準 basE91；升級絕不會改變編碼輸出。Python 套件 `fastbase91` 與 Rust crate `fastbase91-core` 共用同一個版號。

## 0.1.2

- **編碼**改寫成一次處理多個位元組（word-at-a-time）的熱迴圈，並以一張成對查表輔助。
- **寬鬆解碼**新增了 8-byte 區塊快速路徑，而且在落單符號被消化掉後會立刻重新啟用——所以帶換行、串流的輸入也吃得到加速。
- 輸出與先前版本完全相同；`no_std`、MSRV 1.81 與 `#![forbid(unsafe_code)]` 都維持不變。見[效能測試](benchmarks.md)。

## 0.1.1

- **編碼**熱迴圈改成直線式。
- **解碼**拆成寬鬆（預設）與嚴格兩條路徑，加速了預設路徑。

## 0.1.0

- 首次發佈：Python 套件 `fastbase91` 與 `no_std` Rust crate `fastbase91-core`。
