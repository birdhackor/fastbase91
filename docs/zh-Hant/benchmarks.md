# 效能測試

吞吐量高度取決於機器，以及機器當下在忙什麼，所以每個數字都請當成**相對值**，不是保證。這是在一台電腦上的單次測量；跑跟跑之間有 10–15% 的波動是正常的。

## 測試環境

- Apple M1、macOS 26.4.1
- CPython 3.13.15
- fastbase91 0.1.2（來自 PyPI）
- pybase91 0.2.2（來自 PyPI）

這次測試也確認 fastbase91、pybase91 與純 Python 參考實作產生相同、可互通的 basE91。

## 單執行緒吞吐量

每張表都把 **encode** 與 **decode** 分開，而 decode 再分成**寬鬆（lenient，預設，會略過非字母表位元組）**與**嚴格（strict，會拒絕它們）**。嚴格解碼是 fastbase91／fastbase91-core 的功能；pybase91 0.2.2 與純 Python 基準只有一種解碼模式，嚴格欄以「—」表示。

**1 KiB 輸入**

| 實作 | encode（MiB/s） | decode 寬鬆（MiB/s） | decode 嚴格（MiB/s） |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,152 | 837 | 437 |
| pybase91 | 754 | 621 | — |
| 純 Python | 6.1 | 4.7 | — |

**64 KiB 輸入**

| 實作 | encode（MiB/s） | decode 寬鬆（MiB/s） | decode 嚴格（MiB/s） |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,375 | 948 | 469 |
| pybase91 | 975 | 827 | — |
| 純 Python | 6.3 | 4.9 | — |

**1 MiB 輸入**

| 實作 | encode（MiB/s） | decode 寬鬆（MiB/s） | decode 嚴格（MiB/s） |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,624 | 1,076 | 488 |
| pybase91 | 1,046 | 850 | — |
| 純 Python | 6.4 | 5.5 | — |

嚴格解碼會逐位元組對照字母表，所以比寬鬆解碼慢；它仍遠高於純 Python 基準。純 Python 的數字是受 GIL 限制的 Python bytecode。

## 並行擴展

**這張表量的是 encode**，64 KiB，數字是跨執行緒的總吞吐量 MiB/s，在有 GIL 的 CPython 3.13 上。encode 與 decode 對 1,024 bytes 以上的輸入都會放掉 GIL，所以 decode 的擴展方式一樣；這裡用 encode 當代表。

| 實作 | 1 執行緒（MiB/s） | 2 執行緒（MiB/s） | 4 執行緒（MiB/s） |
| --- | ---: | ---: | ---: |
| fastbase91 | 1,680 | 3,200 | 6,141 |
| pybase91 | 1,038 | 1,039 | 1,043 |
| 純 Python | 7 | 7 | 7 |

fastbase91 在這些 64 KiB 呼叫期間放掉 GIL，於是多條執行緒的 Rust 工作重疊，總吞吐量隨執行緒數成長。整個呼叫都握著 GIL 的實作則持平，因為它的執行緒得輪流；純 Python 版不論如何都卡在受 GIL 限制的 bytecode。
