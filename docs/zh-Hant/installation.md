# 安裝與相容性

## Python

從 PyPI 安裝套件：

```console
python -m pip install fastbase91
```

套件需要 CPython 3.11 以上，只支援 CPython。

### 預先建好的 wheel

| 平台與架構 | 一般 CPython 3.11 以上 | 自由執行緒 CPython 3.14t | 自由執行緒 CPython 3.15t 以上 |
| --- | ---: | ---: | ---: |
| Linux x86_64（glibc 2.28 以上） | ✓ | ✓ | ✓ |
| macOS arm64（Apple silicon、macOS 11 以上） | ✓ | ✓ | ✓ |
| Windows x86_64 | ✓ | ✓ | ✓ |

一般 CPython 使用 abi3 wheel；一顆 wheel 涵蓋 3.11 之後的所有版本，`pip` 會自動挑選相符的 wheel。

### 其他平台

CPython 在 Linux ARM64、Intel Mac、Windows ARM、Alpine／musl，以及 glibc 早於 2.28 的 Linux 等平台沒有預先建好的 wheel。`pip` 會下載原始碼發行版並嘗試在本機編譯；需要目前的 stable 版 Rust 工具鏈（例如透過 rustup 安裝）。CI 只在 Linux x86_64 上驗證原始碼建置。

### 不支援的環境

- PyPy 與其他非 CPython 實作不支援；這些實作沒有 PyPI wheel，CI 也沒有在其上建置或測試。
- 自由執行緒 CPython 3.13t 不支援：沒有 wheel，從原始碼建置會失敗。自由執行緒 CPython 需要 3.14t 以上。
- 子直譯器不支援，匯入成功也不代表受到支援。有自己 GIL 的子直譯器（例如由 `concurrent.interpreters` 建立的）匯入 `fastbase91` 時會拋出 `ImportError`；與主直譯器共用 GIL 的子直譯器（例如嵌入 Python 的應用程式透過 C API 建立的）匯入不會報錯，但擴充的類別與例外型別在各直譯器之間是同一份物件，並未隔離。請在主直譯器使用 `fastbase91`；在子直譯器中使用沒有經過測試。

### 自由執行緒 Python

用自由執行緒直譯器（例如 `python3.14t`）安裝；`pip` 會自動挑選相符的 wheel：

```console
python3.14t -m pip install fastbase91
```

確認匯入後 GIL 仍維持關閉：

```console
python3.14t -c "import sys, fastbase91; print(sys._is_gil_enabled())"
```

GIL 維持關閉時會印出 `False`；若以 `PYTHON_GIL=1` 或 `-X gil=1` 啟動，會開啟 GIL 並印出 `True`。細節見[自由執行緒](free-threading.md)。

### 查看已安裝版本

```console
python -c "import fastbase91; print(fastbase91.__version__, fastbase91.CORE_VERSION)"
```

這兩個值依序是 Python 套件版本與所連結 Rust core 的版本。

## Rust

加入 core crate：

```console
cargo add fastbase91-core
```

最低支援的 Rust 版本是 1.81。

### Features

| Feature 設定 | 可用 API |
| --- | --- |
| `std`（預設；含 `alloc`） | `encode`、`decode`、`encode_into`、`decode_into`、串流 `Encoder`／`Decoder`、`max_encoded_len` 與 `max_decoded_len` |
| 只有 `alloc` | `encode`、`decode`、`encode_into`、`decode_into`、串流 `Encoder`／`Decoder`、`max_encoded_len` 與 `max_decoded_len` |
| 都不開（`no_std`、不需要配置器） | `encode_into`、`decode_into`、串流 `Encoder`／`Decoder`、`max_encoded_len` 與 `max_decoded_len` |

CI 會檢查 `thumbv7em-none-eabihf` 裸機目標的建置。Feature 設定與 Rust 程式範例請見[使用方式](usage.md)。

## 版本

PyPI 上的 Python 套件 `fastbase91` 與 crates.io 上的 Rust crate `fastbase91-core` 各自獨立編版號。Python wheel 內含的 core 版本可由 `CORE_VERSION` 取得；版本紀錄見[變更紀錄](changelog.md)。
