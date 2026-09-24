# 效能測試

[English](../en/benchmarks.md)

以下是儲存庫既有的效能測試範例結果，不是本次重新測量的數字。吞吐量會隨機器與執行環境改變，因此應用於相對比較，不應視為絕對保證。

## 測試環境

- Apple Silicon macOS
- CPython 3.13
- fastbase91 0.1.0
- pybase91 0.2.2

所有吞吐量均以輸入資料的 MiB/s 計算。正確性測試也確認 fastbase91、pybase91 與純 Python 實作會產生相同且可互通的 basE91 資料。

## 單執行緒吞吐量

| 實作 | 1 KiB encode/decode | 64 KiB encode/decode | 1 MiB encode/decode |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 402 / 424 | 450 / 473 | 453 / 475 |
| pybase91 (Rust) | 903 / 754 | 1,038 / 876 | 1,049 / 875 |
| 純 Python | 7.3 / 5.3 | 7.2 / 5.4 | 7.2 / 5.3 |

在這份測試結果的每一列中，pybase91 的單執行緒吞吐量都較高。

## 並行擴展

在有 GIL 的 CPython 3.13 上，以 64 KiB 資料編碼；數字是總吞吐量 MiB/s：

| 實作 | 1 執行緒 | 2 執行緒 | 4 執行緒 |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 452 | 872 | 1,675 |
| pybase91 (Rust) | 946 | 913 | 954 |
| 純 Python | 7 | 7 | 7 |

fastbase91 會在這些 64 KiB one-shot 呼叫期間釋放 GIL，因此多個執行緒的 Rust 計算可以重疊，總吞吐量也隨之成長；在四執行緒結果中超越 pybase91。pybase91 在受測呼叫中持有 GIL，因此數字大致持平；純 Python 實作則受限於持有 GIL 的 bytecode。

這些結果呈現的是取捨，而不是所有情況下的統一排名：在此環境中，pybase91 的單執行緒表現領先；當工作負載能同時執行數個足夠大的呼叫時，fastbase91 則能從並行獲益。
