# 自动更新与发布方案

算码工具（Tauri v2 桌面应用）的自动更新、跨平台构建与发布机制。

> 一句话：**用 GitHub Actions 免费的 mac/Windows runner 构建签名，产物发布到 CNB Release，更新检查与下载全程走 CNB（国内）。GitHub 只是构建农场。**

---

## 1. 架构总览

```
开发者 push tag v* 到 GitHub
        │
        ▼
GitHub Actions（.github/workflows/release.yml）
  ├─ macos-14 (arm64)  ┐ tauri-action 构建 + minisign 签名
  ├─ windows-latest    ┘ → 发 GitHub Release（中转/备份）
  └─ publish-to-cnb：scripts/publish-cnb.sh
        ├─ 下载产物
        ├─ REST 上传到 CNB Release（.app.tar.gz / -setup.exe / .dmg）
        └─ 生成 latest.json（指向 CNB 直链）→ 推回 CNB main
        │
        ▼
用户端 app（tauri-plugin-updater）
  ├─ 检查：读 CNB 的 latest.json（git raw）
  └─ 下载：CNB Release 公开直链 → 验签 → 安装 → 重启
```

| 环节 | 位置 |
|------|------|
| 构建（mac/win 签名包） | GitHub Actions（公共 runner） |
| 二进制托管 + 下载 | **CNB Release**（公开仓库直链，国内） |
| 更新清单 latest.json（endpoint） | **CNB** git raw（国内） |
| 主仓库 | CNB（`origin`），GitHub（`github`）为镜像/构建农场 |

为什么这么分：CNB 公共构建节点**只有 Linux**，出 mac/win 包得靠 GitHub 免费 runner；但下载要走国内，所以成品回 CNB。

---

## 2. 关键组成

**应用内（Tauri）**
- 插件：`tauri-plugin-updater`、`tauri-plugin-process`（Rust，`src-tauri/src/lib.rs` 注册；updater 在 `#[cfg(desktop)]` setup 钩子里）；JS `@tauri-apps/plugin-updater`、`@tauri-apps/plugin-process`。
- 权限：`src-tauri/capabilities/default.json` 加 `updater:default`、`process:default`。
- 配置：`src-tauri/tauri.conf.json`
  - `plugins.updater.endpoints` = `https://cnb.cool/dachengzhihui/keyan-calculator/-/git/raw/main/.updater/latest.json`
  - `plugins.updater.pubkey`（minisign 公钥）
  - `plugins.updater.windows.installMode = "passive"`
  - `bundle.createUpdaterArtifacts = true`
- 前端逻辑：`src/lib/updater.ts`（`check()` → 提示 → 下载进度 → 安装 → `relaunch()`）。启动静默检查在 `App.tsx`；手动检查按钮在状态栏（`status-bar.tsx`，点版本号触发）。

**签名密钥（minisign，每项目独立）**
- 私钥：`~/.tauri/keyan-calculator.key`（**无密码**）。⚠️ 丢失 = 再也无法签更新包，已装用户收不到更新。**务必备份。**
- 公钥写在 `tauri.conf.json`。

**发布脚本（`scripts/`）**
- `sync-version.mjs <ver>`：版本号同步到 package.json / tauri.conf.json / Cargo.toml / Cargo.lock。
- `publish-cnb.sh <tag> <dir>`：REST 建 CNB Release + 上传产物 + 生成 latest.json（CNB 直链）+ 推回 CNB main。CI 与本地通用（靠 `CNB_TOKEN`）。
- `release.sh` / `update-manifest.mjs`：本地手动出包/回填清单（应急、无 CI 时用）。

**CI（`.github/workflows/release.yml`）**：见架构图。

**GitHub Secrets**
| Secret | 用途 |
|--------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | minisign 私钥全文，给 tauri-action 签名 |
| `CNB_TOKEN` | CNB 访问令牌（`repo-code:rw`）。既作 OpenAPI Bearer 传附件，也作 git 推 latest.json 的密码 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 不用设（密钥无密码） |

---

## 3. 发布一个新版本

```bash
node scripts/sync-version.mjs 0.3.0
git commit -am "release 0.3.0"
git tag v0.3.0
git push origin main          # 推 CNB（主仓库）
git push github v0.3.0         # 推 tag 到 GitHub，触发 Actions
```

> CNB 的 tag 由 `publish-cnb.sh` 在发布时创建，**不要预先 `git push origin --tags`**（会提前触发 codewiki）。

之后全自动：GitHub 出 mac+win 签名包（含 dmg）→ 传 CNB Release → 生成 CNB 清单推回 CNB main。

发布后 CNB 上每个版本含三个产物：
- `keyan-calculator_<ver>_aarch64.dmg` — macOS 首次安装
- `keyan-calculator_<ver>_x64-setup.exe` — Windows 安装（也是更新包）
- `keyan-calculator_aarch64.app.tar.gz` — macOS 自动更新包（仅它进 latest.json）

**bot 会往 CNB main 推 latest.json 提交**，发版后本地记得 `git pull --ff-only origin main`（并 `git push github main`）保持三处一致，否则下次本地推送可能把 bot 的清单覆盖回旧版。

---

## 4. CNB Release REST 上传流程（publish-cnb.sh 内部）

base `https://api.cnb.cool/<slug>`，所有请求带 `Authorization: Bearer <CNB_TOKEN>` 和 `Accept: application/json`（**缺 Accept 报 406**）。

1. `POST /-/releases` —— body 必含 `target_commitish`（否则 400），返回 `id`
2. `POST /-/releases/<id>/asset-upload-url`（`asset_name`/`size`/`overwrite`）→ 返回 `upload_url` + `verify_url`
3. `PUT <upload_url>` 上传文件
4. `POST <verify_url>`（已含 token/asset_path/ttl）确认

下载公开直链（公开仓库免鉴权）：`https://cnb.cool/<slug>/-/releases/download/<tag>/<file>`

> 注意区分：`cnb releases get-releases-asset` 那个 API 是**鉴权 + 12h/10 次临时链**，别拿它当下载源；上面这个 web 直链才是稳定公开的。

---

## 5. 踩坑记录（都已处理）

1. **Windows WiX/MSI 打包失败**（`light.exe` 出错）→ Windows 只出 NSIS：`args: --target x86_64-pc-windows-msvc --bundles nsis`。NSIS 本就是 updater 在 Windows 用的格式。
2. **中文 productName 被 GitHub 资源名 sanitize**：非 ASCII 被剥掉，`算码工具_aarch64.app.tar.gz` 变成 `_aarch64.app.tar.gz`（下划线开头）→ 把 `productName` 改成 ASCII `keyan-calculator`（窗口标题/应用内仍中文「算码工具」）。
3. **tauri-action `includeUpdaterJson` 的清单指向 GitHub**：我们要 CNB 直链，所以 `publish-cnb.sh` **自己生成 latest.json**（按文件后缀取产物名，对改名稳健）。
4. **CNB REST**：`Accept` 头必填（406）、建 release 必带 `target_commitish`（400）。
5. **曾误判 CNB Release 无稳定直链** —— 实际公开仓库有（见上）；之前只看了鉴权 API。

---

## 6. 维护红线（动了会让自动更新失效）

- ❌ 换 **minisign 签名密钥**（公钥）—— 老 app 内置旧公钥，验不过新包。
- ❌ 改 **bundle identifier**（`cn.com.dachengzhihui.calculator`）—— app 唯一身份。
- ❌ 改/失效 **endpoint URL** —— 老 app 指向它。
- ⚠️ **CNB 仓库必须保持公开** —— 下载直链免鉴权依赖这个。
- 改 **productName** 不会让更新失效（机制和打包都不依赖它），但 Windows 上改名可能产生重复安装入口；mac 就地替换、影响较小。

---

## 7. macOS Gatekeeper（已知限制，待办）

未做 Apple 签名+公证时，从 dmg 装会报 **"已损坏，无法打开"**（Gatekeeper 对未签名+隔离标记的误导提示，不是真损坏）。

- 临时绕过：`sudo xattr -dr com.apple.quarantine /Applications/keyan-calculator.app`
- 根治：Apple Developer 账号（$99/年）+ Developer ID 签名 + 公证，配 `APPLE_*` Secrets 接进 tauri-action。
- 加签名公证后**现有用户无需重装**：自动更新会推签名版（minisign 密钥不变即可），更新后不再被拦；新用户下签名 dmg 直接可开。

---

## 8. 相关链接

- 主仓库（CNB）：https://cnb.cool/dachengzhihui/keyan-calculator
- 构建农场（GitHub）：https://github.com/weixiaospace/keyan-calculator
- 更新 endpoint：https://cnb.cool/dachengzhihui/keyan-calculator/-/git/raw/main/.updater/latest.json
