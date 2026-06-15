#!/usr/bin/env bash
# 把构建好的更新包发布到 CNB Release，并生成 latest.json（指向 CNB 公开直链）推回 CNB main。
# 下载与检查更新均走 CNB（国内）；GitHub 仅作构建农场。
#
# 依赖：curl jq git；环境变量 CNB_TOKEN（CNB 访问令牌，repo-code:rw）
# 用法：CNB_TOKEN=xxx scripts/publish-cnb.sh <tag> <artifacts-dir>
#   artifacts-dir 内需含：*.app.tar.gz(+.sig)、*-setup.exe(+.sig)
set -euo pipefail

TAG="${1:?用法: publish-cnb.sh <tag> <artifacts-dir>}"
DIR="${2:?缺少 artifacts 目录}"
: "${CNB_TOKEN:?需要环境变量 CNB_TOKEN}"

REPO="dachengzhihui/keyan-calculator"
API="https://api.cnb.cool/$REPO"
DL="https://cnb.cool/$REPO/-/releases/download/$TAG"
VERSION="${TAG#v}"
AUTH=(-H "Authorization: Bearer $CNB_TOKEN" -H "Accept: application/json")

mac_targz=$(cd "$DIR" && ls | grep -E '\.app\.tar\.gz$' | head -1)
win_exe=$(cd "$DIR" && ls | grep -E -- '-setup\.exe$' | head -1)
[[ -n "$mac_targz" && -n "$win_exe" ]] || { echo "缺少 mac/.app.tar.gz 或 win/-setup.exe 产物" >&2; exit 1; }

echo "→ 创建 CNB Release $TAG"
rid=$(curl -fsS -X POST "$API/-/releases" "${AUTH[@]}" -H "Content-Type: application/json" \
  -d "$(jq -n --arg t "$TAG" '{tag_name:$t,target_commitish:"main",name:$t,body:("算码工具 "+$t),draft:false,make_latest:"true"}')" \
  | jq -r '.id')
[[ -n "$rid" && "$rid" != "null" ]] || { echo "创建 release 失败（同名 tag 是否已存在？）" >&2; exit 1; }

# 上传一个二进制到 CNB Release：申请上传 URL → PUT → 确认
upload() {
  local file="$1" name="$2" sz resp up vr
  sz=$(wc -c < "$file" | tr -d ' ')
  echo "→ 上传 $name ($sz B)"
  resp=$(curl -fsS -X POST "$API/-/releases/$rid/asset-upload-url" "${AUTH[@]}" -H "Content-Type: application/json" \
    -d "$(jq -n --arg n "$name" --argjson s "$sz" '{asset_name:$n,size:$s,overwrite:true}')")
  up=$(echo "$resp" | jq -r '.upload_url')
  vr=$(echo "$resp" | jq -r '.verify_url')
  curl -fsS -X PUT -T "$file" "$up" -o /dev/null
  curl -fsS -X POST "$vr" "${AUTH[@]}" -o /dev/null
}

upload "$DIR/$mac_targz" "$mac_targz"
upload "$DIR/$win_exe" "$win_exe"

echo "→ 生成 latest.json（指向 CNB 直链）"
jq -n \
  --arg version "$VERSION" \
  --arg notes "算码工具 $TAG" \
  --arg pub "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --rawfile msig "$DIR/$mac_targz.sig" --arg murl "$DL/$mac_targz" \
  --rawfile wsig "$DIR/$win_exe.sig" --arg wurl "$DL/$win_exe" \
  '{version:$version, notes:$notes, pub_date:$pub, platforms:{
     "darwin-aarch64":{signature:($msig|rtrimstr("\n")),url:$murl},
     "windows-x86_64":{signature:($wsig|rtrimstr("\n")),url:$wurl}}}' > /tmp/latest.json
cat /tmp/latest.json

echo "→ 推送 latest.json 到 CNB main"
tmp=$(mktemp -d)
git clone --depth 1 "https://cnb:${CNB_TOKEN}@cnb.cool/$REPO.git" "$tmp/repo"
mkdir -p "$tmp/repo/.updater"
cp /tmp/latest.json "$tmp/repo/.updater/latest.json"
git -C "$tmp/repo" config user.name "release-bot"
git -C "$tmp/repo" config user.email "release-bot@users.noreply.cnb.cool"
git -C "$tmp/repo" add .updater/latest.json
if git -C "$tmp/repo" diff --cached --quiet; then
  echo "manifest 无变化"
else
  git -C "$tmp/repo" commit -m "chore(updater): 发布 $TAG"
  git -C "$tmp/repo" push origin HEAD:main
fi
echo "✓ 发布完成：$DL/{$mac_targz,$win_exe}"
