#!/usr/bin/env bash
# 手动发布：同步版本 → 签名构建 → 定位产物 → 写入本平台的 latest.json 条目
#
# 前置：
#   - 已生成签名密钥：~/.tauri/keyan-calculator.key（公钥已写入 tauri.conf.json）
#   - 在 macOS 上出 darwin-aarch64 包；在 Windows 上出 windows-x86_64 包
#
# 用法：
#   scripts/release.sh <version> [更新说明]
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

VERSION="${1:?用法: scripts/release.sh <version> [更新说明]}"
NOTES="${2:-Release v$VERSION}"
KEY_PATH="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/.tauri/keyan-calculator.key}"

[[ -f "$KEY_PATH" ]] || { echo "找不到签名私钥：$KEY_PATH" >&2; exit 1; }

echo "→ 同步版本号 $VERSION"
node scripts/sync-version.mjs "$VERSION"

echo "→ 签名构建（tauri build）"
TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_PATH")" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
  pnpm tauri build

BUNDLE="src-tauri/target/release/bundle"
case "$OSTYPE" in
  darwin*)
    PLATFORM="darwin-aarch64"
    ARTIFACT="$BUNDLE/macos/keyan-calculator.app.tar.gz"
    ;;
  msys* | cygwin* | win32)
    PLATFORM="windows-x86_64"
    ARTIFACT="$BUNDLE/nsis/keyan-calculator_${VERSION}_x64-setup.exe"
    ;;
  *) echo "不支持的平台：$OSTYPE" >&2; exit 3 ;;
esac
SIG="$ARTIFACT.sig"

[[ -f "$ARTIFACT" && -f "$SIG" ]] || { echo "找不到更新产物：$ARTIFACT(.sig)" >&2; exit 3; }

echo
echo "================ 构建完成 ================"
echo "平台:     $PLATFORM"
echo "安装包:   $ARTIFACT"
echo "签名文件: $SIG"
echo
echo "下一步（手动）："
echo "  1. 在 CNB 为该版本建 Release，上传上面的安装包，拿到下载直链 <URL>"
echo "  2. 写入 manifest（保留其它平台条目）："
echo "     node scripts/update-manifest.mjs --version $VERSION \\"
echo "          --notes \"$NOTES\" --platform $PLATFORM \\"
echo "          --sig \"$SIG\" --url <URL>"
echo "  3. 提交并推送 .updater/latest.json 到 main（endpoint 读取该文件）"
echo "=========================================="
