#!/usr/bin/env node
// 用本平台的构建产物信息更新 .updater/latest.json（保留其它平台条目）
// 用法：
//   node scripts/update-manifest.mjs --version 0.2.0 --notes "更新说明" \
//        --platform darwin-aarch64 --sig <sig文件路径> --url <下载直链>
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const manifestPath = resolve(root, '.updater/latest.json');

const args = {};
for (let i = 2; i < process.argv.length; i += 2) {
  args[process.argv[i].replace(/^--/, '')] = process.argv[i + 1];
}

const { version, notes = '', platform, sig, url } = args;
if (!version || !platform || !sig || !url) {
  console.error('缺少参数：--version --platform --sig <文件> --url 必填，--notes 可选');
  process.exit(2);
}

const signature = readFileSync(resolve(sig), 'utf8').trim();
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

manifest.version = version;
manifest.notes = notes;
manifest.pub_date = new Date().toISOString();
manifest.platforms = manifest.platforms ?? {};
manifest.platforms[platform] = { signature, url };

writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
console.log(`✓ 已更新 .updater/latest.json：${platform} → v${version}`);
