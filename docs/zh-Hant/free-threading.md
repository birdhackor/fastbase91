# 自由執行緒

**先講結論：** 如果你為了真正的平行處理才換到自由執行緒 Python，只要匯入一個沒準備好的套件，CPython 就會把整個行程的 GIL 重新打開。CPython 會印出一則 `RuntimeWarning`，但匯入不會失敗，程式會繼續執行；執行緒裡的 Python 程式碼又會被序列化。警告很容易被忽略，最後只會發現程式變慢。fastbase91 特別設計成：匯入它絕不會造成這種情況。

## 它幫你避開的陷阱

自由執行緒 CPython 從 3.13 開始提供，當時是實驗性版本；3.14 起獲得官方正式支援。一般 CPython 有一把全域直譯器鎖（GIL），同一時間只允許一條執行緒跑 Python；no-GIL 版本把它拿掉，讓多條執行緒真的能同時跑。fastbase91 支援自由執行緒 CPython 3.14t 以上；3.13t 不支援，詳見[安裝與相容性](installation.md)。

問題在這裡。編譯型擴充（帶有 C 或 Rust 部分的套件，例如 NumPy，或這個套件）必須「主動宣告」自己可以在沒有 GIL 的情況下執行。如果你匯入一個**沒有**宣告支援的擴充，CPython 為了安全，會把 **GIL 重新打開，而且是整個行程都打開**——不只是那個套件——並印出一則 `RuntimeWarning`；匯入不會失敗，程式會繼續執行。因此只要有一個還沒跟上的相依套件，就可能讓執行緒裡的 Python 程式碼又變成一次一條在跑。

這則警告很容易被忽略，例如輸出被導走、淹沒在日誌裡，或被 `-W ignore` 濾掉（若以 `-W error` 啟動，警告會變成例外，匯入反而會失敗）。執行緒照跑、結果也照樣正確，但執行緒裡的 Python 程式碼又變回一次只跑一條。fastbase91 輸入至少 1,024 位元組的一次性呼叫會放掉 GIL，仍可重疊執行；較小的一次性呼叫，以及所有串流的 `update()` 與 `finish()` 呼叫都會輪流——詳見下方「哪些呼叫會平行跑」一節。

`PYTHON_GIL=1` 或 `-X gil=1` 也可以重新開啟 GIL。反過來，以 `PYTHON_GIL=0` 或 `-X gil=0` 啟動時，即使匯入這類擴充，GIL 也會維持關閉：CPython 會略過上面的退回動作、不印警告，那個擴充就在沒有 GIL 的情況下執行，風險自負。用 `sys._is_gil_enabled()` 確認目前狀態。

## 為什麼匯入 fastbase91 是安全的

Python binding 標記了 `#[pymodule(gil_used = false)]`——這是 Rust 原始碼裡的一行，告訴 CPython「這個模組在沒有 GIL 時是安全的」。因為有這個標記，匯入 fastbase91 **不會**觸發上面那個退回動作；其他沒有宣告支援的擴充，或 `PYTHON_GIL=1`／`-X gil=1`，仍可能重新開啟 GIL。

講具體一點：你在自由執行緒 Python 上做了一個服務，讓八條工作執行緒平行編碼各自的資料，並用 fastbase91 來做編碼。匯入它之後，`sys._is_gil_enabled()` 維持 `False`，你的八條執行緒繼續同時跑。要是你當初挑的是一個沒宣告支援、而且工作時握著 GIL 的擴充，光是那一次匯入就會把 GIL 打開，八條執行緒就得排隊輪流跑——輸出一樣，吞吐量只剩一小部分。

有一個持續整合（CI）工作在每次改動時都會驗這件事：在自由執行緒直譯器上匯入 fastbase91，斷言 `sys._is_gil_enabled()` 仍是 `False`，接著把大量資料丟進並行的 encode/decode 呼叫做 round-trip，逐位元組檢查。（這證明的是匯入安全與並行正確性，並不是保證「多條執行緒共用同一個 `Encoder`/`Decoder` 物件」也安全；見下方。）

## 我們發行的 wheel

除了常規的 abi3 wheel（涵蓋 CPython 3.11 以上），我們也為 Linux x86_64、macOS arm64（Apple silicon）與 Windows x86_64 發行給自由執行緒直譯器用的 wheel；完整平台表見[安裝與相容性](installation.md)：

| Wheel | 適用於 |
| --- | --- |
| `cp314-cp314t` | 自由執行緒 CPython 3.14（也就是「3.14t」版本） |
| `cp315` ＋ `abi3t` ABI | 自由執行緒 CPython 3.15t 及後續版本（它也帶 `abi3` 標籤，所以一般 CPython 3.15 以上也能安裝） |

兩者在 0.1.1 都已上架 PyPI。`pip install fastbase91` 會自動依你的直譯器挑對的那顆。

## 哪些呼叫會平行跑

`fastbase91.encode()` 與 `fastbase91.decode()` 會依你給的輸入位元組數，逐次決定要不要放掉 GIL：

| 輸入大小 | 會發生什麼 |
| --- | --- |
| 1,024 bytes 以上 | 放掉 GIL（從 Python `detach`），期間由 Rust 做事 |
| 小於 1,024 bytes | 保留 GIL——放掉再重新取得的成本，會比省下來的還多 |

這對 **encode 與 decode 都適用**，而且是以你傳入的輸入位元組數為準，不是編碼／解碼後的大小。

在一般（有 GIL 的）CPython 上，正是「大型呼叫放掉 GIL」讓不同執行緒的 Rust 工作可以重疊，所以總吞吐量會隨執行緒數增加——實測數字見[效能測試](benchmarks.md)。1 KiB 以下的呼叫刻意保留 GIL，所以別期待小輸入也有這種擴展。

## 實際上怎麼用這些執行緒

平行工作的單位是一次獨立的 one-shot 呼叫，而且輸入在 1 KiB 以上時效益最好：

```python
from concurrent.futures import ThreadPoolExecutor
import fastbase91

payloads = [b"x" * 64_000 for _ in range(8)]
with ThreadPoolExecutor(max_workers=4) as pool:
    encoded = list(pool.map(fastbase91.encode, payloads))
```

串流物件則不一樣。`Encoder` 與 `Decoder` 持有可變狀態，所以請讓**每條執行緒（或每個串流）都有自己的實例**——絕不要跨執行緒共用同一個。從兩條執行緒同時對同一個物件呼叫 `update()`，並不是一種同步手段，還可能拋出 `RuntimeError`。

還有，如果你的輸入由可寫入的記憶體支援（例如 `bytearray`），呼叫正在讀取時，請勿讓別條執行緒去改那塊 buffer。
