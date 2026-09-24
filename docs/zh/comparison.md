# 方案比較

[English](../en/comparison.md)

本頁比較的 basE91 實作都採用標準演算法。fastbase91、pybase91 與本儲存庫的純 Python 實作會產生逐位元組相同的輸出，也能互通。

## basE91 實作

| 方案 | 優點 | 取捨 |
| --- | --- | --- |
| **fastbase91** | 提供 free-threaded 與 Windows wheel；大型 one-shot 呼叫會釋放 GIL；可重用的 `no_std` Rust core；支援串流與 strict 解碼；Rust core 禁止 unsafe code | 在本比較中，單執行緒吞吐量不是最高 |
| **pybase91** | Rust/PyO3 實作提供優異的單執行緒吞吐量 | 本次比較的版本支援 macOS/Linux 與 Python 3.11–3.13，沒有 free-threaded 或 Windows wheel |
| **純 Python** | 沒有編譯相依性，原始碼可攜性最高 | 在這些測量中約比 Rust 實作慢 50–180 倍 |

若要在 pybase91 支援的 Python 與平台組合上追求最高單執行緒吞吐量，pybase91 是很好的選擇。fastbase91 則著重 free-threaded 執行、大型呼叫的跨執行緒擴展、Windows 發行、可重用的 `no_std` core、安全 Rust、串流與 strict 解碼。

## 其他 binary-to-text 編碼

| 編碼 | 約略體積膨脹 | 說明 |
| --- | ---: | --- |
| base64 | 33% | 支援廣泛；Python 標準庫內建 |
| base85 | 25% | 比 base64 緊湊；Python 標準庫內建 |
| basE91 | 23% | 三者中最緊湊；91 字元的字母表使用較多標點符號 |

體積數字是近似值，也會受輸入長度影響。base64 與 base85 是不同格式，而不是 basE91 的其他實作，因此其編碼資料無法與 basE91 互換。

請依周邊協定與執行環境需求選擇：互通性與生態系普及度可能比密度更重要，而並行能力、平台 wheel 或 `no_std` 支援也可能決定實作選擇。

