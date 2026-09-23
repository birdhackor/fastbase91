# 維護與發布 Runbook

## Wheel 與 Python 支援窗

wheel matrix 位於 `.github/workflows/python.yml` 的 `build-wheels` 與
`test-wheels` jobs：一般 CPython 用 `abi3-py311`，CPython 3.14t 用
版本專屬 `cp314-cp314t`，CPython 3.15+ 用 `abi3t-py315`。新增或移除
Python／平台時，必須同步更新這兩個 matrix、實際安裝 smoke test，以及
這份文件。

free-threaded 支援是跟著 PyO3 與 maturin 的可用支援窗走。當 PyO3 不再
支援某個版本專屬 free-threaded ABI 時，不為保留單一 wheel 而凍結舊
PyO3：在 release note 宣告停產，移除該 matrix entry，並保留仍受支援的
`abi3t` 路徑。新的版本專屬 free-threaded wheel 則要先在三平台建置、安裝、
確認 import 不重新啟用 GIL、並通過並行 round-trip gate，才納入 matrix。

## 升級 PyO3

1. 從 Renovate 的 PR 開始，閱讀 PyO3 migration/free-threading 文件與
   maturin 相容性說明；pre-1.0 的 minor 升級也視同可能 breaking，絕不自動合併。
2. 更新 `bindings/python/Cargo.toml` 的 PyO3 範圍並重建 `Cargo.lock`；檢查
   `abi3-py311`、`abi3t-py315` 與 free-threaded feature 是否仍正確。
3. 在 PR 和至少一次 release-candidate 上跑 `cargo test --all-features`、
   `pytest bindings/python/tests -q`，並跑 `.github/workflows/python.yml` 的完整
   三平台 wheel matrix（含 free-threaded 與安裝後測試）。
4. 若新的 CPython 尚未能由 PyO3/maturin 端到端建置，先停在相容的支援窗並
   記錄上游 issue；不要靠未驗證的 ABI tag 或釘死舊 PyO3 來假裝支援。

## 發布

`release.yml` 在推送 `v<套件版本>` tag 時觸發；保留手動觸發以便從已選取的
tag 做一次完整、全新的發布嘗試。`validate-release-ref` 強制 ref 類型為 tag，且
tag 必須精確符合 `^v[0-9]+\.[0-9]+\.[0-9]+$`：目前只接受三段式穩定版，避免
預發布／post-release 的 Cargo 與 Python 版本正規化產生歧義。它呼叫 `python.yml`
的 `workflow_call`：同一個 workflow run 會建置 9 組 wheels 與 sdist、產生
manifest、各 wheel 安裝測試、從獨立解壓的 sdist 重建 wheel 並做 import／round-trip
smoke test，然後把 manifest 和 distributions 封裝成 `release-batch` artifact。頂層
release job 再以同一 manifest 重驗 filename、SHA-256、source commit 與 tag 版本，
成功後才允許發布工作開始。

一個 release 必須完全來自同一個 immutable `release-batch`。PyPI 發布絕不使用
`skip-existing`，也沒有自動重建補檔路徑；手動觸發也一樣會建立與上傳完整的新 batch。
`publish-pypi` 與 `publish-crates` 串成固定順序（crates 在 pypi 成功後才跑），且以
`concurrency` group 綁 tag，使同一 tag 一次只跑一個 release run；因此兩個 registry
不可能由不同 run 各贏一個。任何部分發布失敗只能走下面的人工復原，不得靠重跑補齊
（重跑時 pypi 會因既有檔案失敗，crates 隨其上游一併被 skip）。
在開始發布前，必須同步提升 core crate、Python binding crate 與
`bindings/python/pyproject.toml` 所代表的套件版本，並確認三者與 tag 相同。
`publish-crates` 會由 `cargo metadata` 讀取 `fastbase91-core` 的實際版本，與 tag 去掉
`v` 後的版本逐字比較，不符即停止。將 `.crate` 一併納入 manifest 做遠端 hash 對照
是 P10/P11 可再加強的項目；目前的必要 gate 是 core version 綁定 tag。

PyPI Trusted Publisher 必須綁定下列值（P11 設定）：GitHub owner、repository、
workflow filename **`release.yml`**，以及 GitHub protected environment **`pypi`**。
若 PyPI 專案尚不存在，pending publisher 也要使用完全相同的 owner、repository、
workflow filename 和 environment。`publish-pypi` 是頂層 workflow 中唯一持有
`id-token: write` 的 job，刻意不配置 PyPI API token。

`publish-crates` 只發布 `fastbase91-core`，在 protected environment `crates` 使用
`CARGO_REGISTRY_TOKEN` 完成首次 crates.io 發布；首發後依 crates.io 當時的 OIDC
設定改用信任式身分。不得發布 `fastbase91-python`：它對 core 的 path dependency
尚未給 `version`。待 P11 補上 version、完成 package/dry-run 審查後，才能另行
設計 binding crate 的發布流程。

## 發布後與補救

1. 下載 PyPI 與 crates.io 的實際檔案清單，逐一對照 release artifact 的
   `manifest.json` filename 與 SHA-256，並保存核對結果到 release issue 或紀錄。
2. 若部分發布失敗，**不得**以重建 batch 後跳過既有檔案來補檔。先停止所有發布；
   可選擇將該版本在每個已發布 registry 全數 yank（crates.io：
   `cargo yank --vers <版本> fastbase91-core`；PyPI：使用當前的 release/yank 管理
   介面），同步更新三個版本後建立新 tag 並做全新發布；或只人工上傳原始、immutable
   `release-batch` 中尚未上傳的檔案，且須先逐一核對遠端檔案與 manifest 的 SHA-256。
   絕不可覆寫同名檔案，亦不可用新建 batch 與已存在檔案混成同一版本。
3. 若有錯檔、錯 hash 或安全問題，停止補檔並將受影響版本全數 yank；已發布檔案
   不得替換，改以新版本與新 tag 發布。

## Weekly canary 與 dead-man's switch

`.github/workflows/canary.yml` 每週三 04:23 UTC 跑兩條路：以 `3.16-dev`
安裝已發布的 binary wheel，以及 `cargo update` 與允許範圍內最新 maturin
重建、安裝、執行測試。GitHub 的 scheduled workflows 只在 default branch
執行，且 public repository 長期無活動可能被停用，因此它不是唯一監測機制。

兩條 canary 都成功後，`ping-dead-mans-switch` 才會 POST 到 secret
`HEALTHCHECKS_PING_URL`（例如 healthchecks.io 的唯一 ping URL）。P11 要把
該 endpoint 的預期週期設為大於一週並啟用逾期通知；任何 canary 失敗、排程
被停用或未執行都不會 ping，外部服務因此會告警。

## P11 上線待辦

- 確認各 package／distribution 所需的版權持有人與 copyright notice。
- 設定 PyPI pending publisher/Trusted Publisher 與 protected `pypi` environment，
  並限制可部署的 tag；確認 trust binding 的 workflow filename 是 `release.yml`。
- 設定 protected `crates` environment 與最小權限的 `CARGO_REGISTRY_TOKEN`；首發後
  依 crates.io OIDC 支援狀態遷移。
- 設定 `HEALTHCHECKS_PING_URL` secret 與外部 dead-man's-switch 告警。
- 在 `fastbase91-python` 的 core path dependency 加入正確的 core `version`、完成
  package/dry-run 審查後，才討論發布 binding crate。
- 實際推送測試 tag／正式 tag 前，確認 GitHub environment protection、PyPI project
  name、crates.io package name和 remote artifact hash 核對流程均已就緒。
