# 算码工具 keyan-calculator

独立的科研数据**算码 + 时间存证**桌面应用（Tauri 2 + React + Rust）。

选文件夹 → 批量计算国密 **SM3** → 生成不可变的**时间存证派生码** → 批量上传。

## 能力

- **选文件夹**：常驻管理多个文件夹，文件树展示算码/上传状态。
- **批量算码**：流式 SM3（任意大文件不爆内存）+ rayon 并行（留一核给 UI）；按 `文件大小 + 修改时间 + 创建时间` 命中缓存，避免重算。
- **时间存证派生码**：`SM3( sm3 | created | modified | calc_ts(毫秒) )`，不可变、append-only，按路径聚合历史。
- **批量上传**：把待传存证 POST 到可配置 endpoint（本地优先、后端无关）。

> v1 时间源为本机时钟（`time_source=local`），做成可插拔，将来可升级为国密可信时间戳（TSA）。

## 技术栈

- 前端：React 19 + Vite 6 + Tailwind v4 + shadcn/ui + next-themes
- 后端：Rust（Tauri 2）+ rusqlite(WAL) + sm3 + jwalk/rayon + reqwest
- 本地 SQLite 为唯一真相源（append-only 存证日志）

## 开发

```bash
pnpm install        # 安装依赖
pnpm dev            # 启动 Tauri 桌面应用（开发）
pnpm dev:vite       # 仅启动前端（浏览器调试）
```

## 构建

```bash
pnpm build          # tsc + vite build + tauri build（生成安装包）
pnpm build:vite     # 仅构建前端
```

## 目录

```
keyan-calculator/
├── src/                # 前端（React）
│   ├── components/     # UI 组件（shadcn/ui + 业务组件）
│   ├── lib/            # tauri 调用封装、格式化
│   └── App.tsx
└── src-tauri/          # Rust 后端
    └── src/
        ├── hasher.rs   # SM3 + 派生码
        ├── scanner.rs  # 文件夹扫描
        ├── store.rs    # SQLite 存证
        ├── uploader 逻辑（commands.rs 内）
        └── commands.rs # Tauri 命令
```

## 测试

```bash
cd src-tauri && cargo test --lib    # 后端单元测试（SM3 向量、派生码、存证逻辑）
```

bundle identifier：`cn.com.dachengzhihui.calculator`
