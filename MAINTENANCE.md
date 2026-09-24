# 維護與發布 Runbook

## Wheel 與 Python 支援窗

wheel matrix 的單一真值源是 `.github/wheel-matrix.json`；
`.github/workflows/python.yml` 的 `build-wheels`、`test-wheels` 與 manifest
完整性 gate 都讀取它。目前發行三層（皆三平台）：一般 CPython 用 `abi3-py311`，
CPython 3.14t 用版本專屬 `cp314-cp314t`，CPython 3.15+ free-threaded 用
`abi3t-py315`。`abi3t-py315` 已於 Python 3.15 進入 RC、ABI 凍結後恢復。

build 與 test 的直譯器來源分開決定。**build** 只有兩種 provider
（`wheel-matrix.json` 的 `python_provider`）：manylinux 各層在 maturin-action 容器內
建置；所有 Windows 與 macOS 層一律由 uv 安裝對應直譯器（regular 的 3.11、
free-threaded 的 3.14t／3.15t）後交給 maturin 的 `-i`。**test** 依 `free_threaded`
決定，與 build provider 無關：free-threaded 層（含 3.15 RC）由 uv 建隔離環境，
regular 層由 `actions/setup-python` 提供。新 stable-ABI wheel 可先做開發驗證，但只在
對應 CPython 進入 RC、ABI 凍結後才納入正式發行 matrix。新增或移除
Python／平台時，必須更新這份 matrix、確認實際安裝 smoke test，以及更新
這份文件。manifest gate 依 artifact 名 `wheels-<id>` 對應 matrix id；每個 id
必須恰有一顆符合宣告 Python／ABI／平台 tag 的 wheel，另須恰有一份 sdist，
其中 compressed ABI tag（例如 maturin 產生的 `abi3.abi3t`）必須包含 matrix 宣告的 ABI，
並拒絕缺項、多項、重複 id、重複 filename 與非預期的 `wheels-*` 目錄。

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

core 與 Python 套件採兩條獨立版本線，首發版號雖然都是 `0.1.0`，後續不要求同步升版：

- Python tag `vX.Y.Z` 只建置並發布 `fastbase91` wheel 與 sdist 到 PyPI。
- core tag `fastbase91-core-vX.Y.Z` 只發布 `fastbase91-core` 到 crates.io。

兩種 tag 都進入單一 `.github/workflows/release.yml`，再由 `validate-release-ref` 的 `kind`
output 分流。ref 必須是 tag，且要精確符合 `^v[0-9]+\.[0-9]+\.[0-9]+$` 或
`^fastbase91-core-v[0-9]+\.[0-9]+\.[0-9]+$`；目前只接受三段式穩定版。保留
`workflow_dispatch`，但手動執行時也必須選取符合規則的 tag。`concurrency` 以完整 ref
分組，使同一 tag 一次只跑一個 release run；兩種 tag 彼此不串接。

### core 發布

1. 只提升 `crates/fastbase91-core/Cargo.toml` 的版本。若 binding 的 dependency lock
   需要更新，執行 `cargo update -p fastbase91-core` 並提交同步後的 `Cargo.lock`。
2. 執行 `cargo package -p fastbase91-core --list`，確認只含預期原始碼、benchmark 與授權檔，
   再完成測試與 dry-run 審查。
3. 在要發布的 commit 建立並推送 `fastbase91-core-vX.Y.Z`。workflow 以
   `cargo metadata` 確認 crate 版本等於 tag 版本，並以單次、fail-closed 的
   `crates-has` 查詢要求該版本尚不存在，然後執行
   `cargo publish -p fastbase91-core --locked`。
4. 等 crates.io 已能查到該版本，再發布任何內含這版 core 的 Python wheel。

`bindings/python/Cargo.toml` 的 binding crate 設為 `publish = false`；它是 Python
`cdylib` 建置單元，永遠不發布到 crates.io。

### Python 發布與 core 先發不變式

1. 提升 `bindings/python/Cargo.toml` 的版本，並執行 `cargo update -p fastbase91-python`
   同步 `Cargo.lock`。CI 的 wheel 建置以 `maturin build --locked` 進行（本機可先用
   `cargo metadata --locked` 預檢），`Cargo.lock` 未同步會直接失敗；升版 commit 必須同時
   包含這兩個檔。`bindings/python/pyproject.toml` 保持
   `dynamic = ["version"]`，Python 套件版本來自 binding Cargo metadata。
2. 若 core 沒有改，只建立並推送 `vA.B.C`。若 core 有改，先依上一節推送 core tag、
   等 crates.io 落地，再推送 Python tag。共同發布時兩個 tag 指向同一 commit，而且
   **先推 core tag**；可以緊接著推 Python tag，wheel 守衛會有界輪詢等待 crates.io。
   release tag 不可移動、版本不可覆寫，所以**務必先把升版 commit 推上 main、等該 commit 的
   `python-ci` 與 `rust-ci` 綠，再對同一 commit 打輕量 tag 並推送**；其中 `python-ci` 的
   wheel 建置以 `maturin build --locked` 進行，等於發布前的 lock 同步預檢（`rust-ci` 不帶
   `--locked`）。tag 一旦推出即綁定該內容，內容有誤只能跳下一個版號。
3. Python 線呼叫 `python.yml` 的 `workflow_call`，建置 9 組 wheels 與 sdist、產生
   manifest、做安裝與 sdist 重建測試，並組成 immutable `release-batch`。頂層 job
   再重驗 filename、SHA-256、source commit 與 Python tag 版本。
4. 發布前的 core 先發守衛由 `crates/fastbase91-core/Cargo.toml` 讀取 core 版本 V，
   並要求三件事全成立：(a) git 有 `fastbase91-core-vV`；(b) HEAD 的
   `crates/fastbase91-core` 子樹與該 tag 無差異；(c) crates.io 已有
   `fastbase91-core` V。缺 tag、子樹不同或有界輪詢後仍不存在皆停止；registry 查詢
   持續出錯也會 fail-closed。前兩項把 wheel 內含的 core 綁定到本地 tag
   `fastbase91-core-vV`，第三項證明該版本已發布——**只要該 tag 未被 force-move，兩者
   合起來即等於「wheel 內含的 core == crates.io 上已發布的 V」。** `immutable release tags`
   ruleset 已啟用，禁止 `fastbase91-core-v*`／`v*` 的 update、delete 與 force-push，
   因此「tag 不可被移動」是由 active ruleset 保證的信任邊界，不再只是假設。
   未來若需再縮小信任邊界，可選擇加上遠端 `.crate` 與本地 core package 的逐檔比對。
5. `publish-pypi` 以單次、fail-closed 的 `pypi-has` 查詢要求 `fastbase91` 的 Python
   版本尚不存在，並以 Trusted Publishing OIDC 上傳完整 `release-batch/artifacts`；
   不使用 token 或 `skip-existing`。
6. 上傳後，同一 job 執行 `pypi-set-matches`。它對 PyPI 的 filename + SHA-256 集合
   做有上限輪詢：缺 release／缺檔可等待傳播，非預期檔或 hash 衝突立即失敗，查詢錯誤
   只在有限預算內重試；逾時會讓 job 失敗並告警。這是發布後核對，不是 core 發布的
   前置條件。

兩個 publish job 都要求 `GITHUB_RUN_ATTEMPT == 1`。發布前「版本必須不存在」的
`pypi-has`／`crates-has` 不做重試，以免暫時查不到被誤判成安全；只有 Python 的
「core 必須已存在」守衛會輪詢 `crates-has`。

PyPI Trusted Publisher 必須綁定下列值（P11 設定）：GitHub owner、repository、
workflow filename **`release.yml`**，以及 GitHub protected environment **`pypi`**。
若 PyPI 專案尚不存在，pending publisher 也要使用完全相同的 owner、repository、
workflow filename 和 environment。`publish-pypi` 是頂層 workflow 中唯一持有
`id-token: write` 的 job，刻意不配置 PyPI API token。

`publish-crates` 在 protected environment `crates` 使用最小權限
`CARGO_REGISTRY_TOKEN`。GitHub environment 的 selected deployment tag 規則必須設為
`pypi` 僅允許 `v*`，`crates` 僅允許 `fastbase91-core-v*`；配合 workflow 分流，確保
每個 secret／OIDC 身分只從自己的 tag 樣式可達。

## 發布後與補救

core 與 Python 是獨立發布線，不構成跨網站交易：一條線成功後，另一條仍可能自行失敗，
也不應因此回滾已成功的另一條線。PyPI 的 `pypi-set-matches` 只驗證該 Python 發布，
不提供跨 registry 原子性。

每條線各自遵守以下復原規則：

1. 版本尚未寫入該 registry 時，可從同一 tag 開一個新的完整 run；不可重跑 publish job。
2. 版本已存在即拒發，絕不覆寫或用重建產物補成同一版本。若內容錯誤或發布不完整，
   在該 registry yank 受影響版本（crates.io：
   `cargo yank --vers <版本> fastbase91-core`；PyPI 使用當前 release/yank 管理介面），
   只提升該套件自己的版號、建立該版本線的新 tag，再做全新發布。
3. 一條線失敗不牽動另一條；但新的 Python 發布仍必須通過 core 先發不變式。

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

- 設定 PyPI pending publisher/Trusted Publisher 與 protected `pypi` environment，
  首發前即限制 deployment tag 為 `v*`；確認 trust binding 的 workflow filename 是 `release.yml`。
- 設定 protected `crates` environment 與最小權限的 `CARGO_REGISTRY_TOKEN`，首發前即
  限制 deployment tag 為 `fastbase91-core-v*`；首發後再依 crates.io OIDC 支援狀態遷移。
- 已完成：active ruleset `immutable release tags` 禁止 `fastbase91-core-v*` 與 `v*`
  的 update、delete 與 force-push，使已發布 tag immutable、core 先發守衛對本地 tag 的信任成立。
- 已完成：active ruleset `main branch CI gate`（bypass：repo admin）要求 `python-ci`
  與 `rust-ci` 兩個 aggregate check 通過才能更新 main；matrix job 名稱會隨 matrix 變動，
  故 required check 綁這兩個固定名稱而非個別 matrix leg。這也讓 GitHub 原生 auto-merge 可用。
- 已完成：Renovate 只對 CI 基礎 Action（checkout／setup-python／download-artifact／
  upload-artifact）的非 major 更新 auto-merge；建置工具（maturin-action／setup-uv／
  rust-toolchain）、發布 Action（gh-action-pypi-publish）與 pre-1.0 相依（PyO3）維持人工。
- 設定 `HEALTHCHECKS_PING_URL` secret 與外部 dead-man's-switch 告警。
- 實際推送測試 tag／正式 tag 前，確認 GitHub environment protection、PyPI project
  name、crates.io package name和 remote artifact hash 核對流程均已就緒。
