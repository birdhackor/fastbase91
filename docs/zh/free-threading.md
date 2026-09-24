# 自由執行緒

[English](../en/free-threading.md)

Python 擴充套件宣告 `#[pymodule(gil_used = false)]`。因此在 free-threaded CPython 上匯入 fastbase91 時，不會重新啟用 GIL。

## 發行的 wheel 規則

- `cp314-cp314t` wheel 適用於 free-threaded CPython 3.14。
- `abi3t-py315` wheel 使用自由執行緒穩定 ABI，適用於 CPython 3.15t 及後續版本。

除了常規 ABI3 wheel，也會為 Linux、macOS 與 Windows 建置 free-threaded wheel。

## One-shot 呼叫

`fastbase91.encode()` 與 `fastbase91.decode()` 會依輸入大小採用不同策略：

| 輸入大小 | 行為 |
| --- | --- |
| 至少 1,024 bytes | Rust 計算期間從 Python detach |
| 小於 1,024 bytes | 保持 attached，省去 detach/reattach 成本 |

在有 GIL 的 CPython 上，大型呼叫 detach 後，不同 Python 執行緒的 Rust 工作可以重疊執行。儲存庫中的效能測試記錄到四執行緒約 1,675 MiB/s 的總編碼吞吐量。小型呼叫刻意保持 attached，因為此時 detach 成本占比較高；1 KiB 以下不應期待同樣的 GIL 釋放擴展效果。

此門檻以每次 one-shot 呼叫收到的輸入位元組數計算，而不是最終編碼或解碼後的大小。

## 建議的執行緒模型

建議以彼此獨立的 one-shot 呼叫為平行工作單位，尤其是輸入至少 1 KiB 時：

```python
from concurrent.futures import ThreadPoolExecutor
import fastbase91

payloads = [b"x" * 64_000 for _ in range(8)]
with ThreadPoolExecutor(max_workers=4) as pool:
    encoded = list(pool.map(fastbase91.encode, payloads))
```

`Encoder` 與 `Decoder` 是持有可變狀態的串流狀態機。每個串流或執行緒都應建立自己的實例，請勿讓多個執行緒同時共用同一實例。同時對同一個 Python 實例呼叫 `update()` 並不是同步機制，而且可能拋出 `RuntimeError`。

若輸入由可寫入的記憶體支援，呼叫正在讀取時請勿從其他執行緒修改該 buffer。

## 持續整合 gate

Free-threaded wheel job 會在 free-threaded interpreter 上驗證以下兩點：

1. 匯入 `fastbase91` 後，`sys._is_gil_enabled()` 仍為 false；
2. 多筆 payload 經並行 encode/decode 呼叫後，仍能正確 round-trip。

這些檢查涵蓋匯入行為與並行正確性，但不代表共用可變串流物件是 thread-safe。

