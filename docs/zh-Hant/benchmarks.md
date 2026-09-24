# 效能測試

這些數字是為 0.1.1 版重新測量的；0.1.1 把編碼改成直線式熱迴圈、並把解碼拆成兩條路（見下文）。吞吐量高度取決於機器，以及機器當下在忙什麼，所以每個數字都請當成**相對值**，不是保證。這是在一台筆電上的單次測量；跑跟跑之間有 10–15% 的波動是正常的。

## 測試環境

- Apple M1、macOS 26.4.1
- CPython 3.13.15
- fastbase91 0.1.1（以本儲存庫原始碼建置）
- pybase91 0.2.2（來自 PyPI）

每個數字都是**輸入**位元組的 MiB/s。這次測試也確認 fastbase91、pybase91 與純 Python 參考實作產生相同、可互通的 basE91。

## 單執行緒吞吐量

每張表都把 **encode** 與 **decode** 分開，而 decode 再分成**寬鬆（lenient，預設，會略過非字母表位元組）**與**嚴格（strict，會拒絕它們）**。嚴格解碼是 fastbase91／fastbase91-core 的功能；pybase91 0.2.2 與純 Python 基準只有一種解碼模式，嚴格欄以「—」表示。

**1 KiB 輸入**

| 實作 | encode | decode（寬鬆） | decode（嚴格） |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 614 | 683 | 480 |
| pybase91 (Rust) | 890 | 737 | — |
| 純 Python | 7.0 | 5.4 | — |

**64 KiB 輸入**

| 實作 | encode | decode（寬鬆） | decode（嚴格） |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 715 | 856 | 565 |
| pybase91 (Rust) | 1,043 | 827 | — |
| 純 Python | 7.4 | 5.4 | — |

**1 MiB 輸入**

| 實作 | encode | decode（寬鬆） | decode（嚴格） |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 743 | 863 | 572 |
| pybase91 (Rust) | 1,048 | 883 | — |
| 純 Python | 7.4 | 5.5 | — |

怎麼讀：pybase91 單執行緒編碼較快（這裡約 1.4–1.5 倍）。寬鬆解碼現在幾乎打平——64 KiB 時 fastbase91 略勝，1 MiB 時 pybase91 略勝。嚴格解碼讓 fastbase91 大約付出寬鬆解碼三分之一的速度，這是「逐位元組檢查」的代價，而它仍約是純 Python 基準的 100 倍。

## 並行擴展

**這張表量的是 encode**，64 KiB，數字是跨執行緒的總吞吐量 MiB/s，在有 GIL 的 CPython 3.13 上。encode 與 decode 對 1,024 bytes 以上的輸入都會放掉 GIL，所以 decode 的擴展方式一樣；這裡用 encode 當代表。

| 實作 | 1 執行緒 | 2 執行緒 | 4 執行緒 |
| --- | ---: | ---: | ---: |
| fastbase91 (Rust) | 733 | 1,407 | 2,704 |
| pybase91 (Rust) | 1,039 | 1,040 | 1,040 |
| 純 Python | 7 | 7 | 7 |

fastbase91 在這些 64 KiB 呼叫期間放掉 GIL，於是多條執行緒的 Rust 工作重疊，總吞吐量幾乎線性成長。它在單執行緒落後 pybase91，兩執行緒追平超車，四執行緒大幅領先。pybase91 持平，因為它整個呼叫都握著 GIL；純 Python 版則不論如何都卡在受 GIL 限制的 bytecode。

重點是取捨，不是排名：pybase91 贏單執行緒編碼，而只要工作負載有幾個大型呼叫能同時跑，fastbase91 就贏。
