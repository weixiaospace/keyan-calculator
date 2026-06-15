#!/usr/bin/env node
// 把版本号同步到 package.json / tauri.conf.json / Cargo.toml / Cargo.lock
// 用法：node scripts/sync-version.mjs <version>
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const version = process.argv[2];

if (!/^\d+\.\d+\.\d+(?:-[\w.]+)?$/.test(version ?? '')) {
  console.error(`版本号格式不合法：${version}（应形如 0.2.0）`);
  process.exit(2);
}

function patch(rel, fn) {
  const path = resolve(root, rel);
  const next = fn(readFileSync(path, 'utf8'));
  writeFileSync(path, next);
  console.log(`✓ ${rel} → ${version}`);
}

// 顶层 JSON 的第一个 "version": 字段
const jsonVersion = (t) =>
  t.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${version}"`);

patch('package.json', jsonVersion);
patch('src-tauri/tauri.conf.json', jsonVersion);
patch('src-tauri/Cargo.toml', (t) =>
  t.replace(/^version = "[^"]*"/m, `version = "${version}"`),
);
patch('src-tauri/Cargo.lock', (t) =>
  t.replace(/(name = "calculator"\nversion = )"[^"]*"/, `$1"${version}"`),
);
