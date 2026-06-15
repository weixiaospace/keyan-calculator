// 应用自动更新：基于 Tauri 官方 updater 插件。
// endpoint 与公钥在 src-tauri/tauri.conf.json 的 plugins.updater 中配置，
// check() 会拉取 latest.json、对比版本并用内置公钥验签。

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { toast } from 'sonner';

let installing = false;

/**
 * 检查更新；若有新版则弹出可操作的提示。
 * @param silent 启动时静默检查：无更新或出错都不打扰用户。
 */
export async function runUpdateCheck(silent: boolean): Promise<void> {
  let update: Update | null = null;
  try {
    update = await check();
  } catch (e) {
    if (!silent) toast.error(`检查更新失败：${e}`);
    return;
  }

  if (!update) {
    if (!silent) toast.success('已是最新版本');
    return;
  }

  toast(`发现新版本 v${update.version}`, {
    description: update.body?.trim() || undefined,
    duration: Infinity,
    action: {
      label: '更新并重启',
      onClick: () => void installUpdate(update as Update),
    },
  });
}

async function installUpdate(update: Update): Promise<void> {
  if (installing) return;
  installing = true;

  const id = toast.loading('正在下载更新…');
  try {
    let downloaded = 0;
    let total = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          total = event.data.contentLength ?? 0;
          break;
        case 'Progress':
          downloaded += event.data.chunkLength;
          toast.loading(
            total > 0
              ? `下载中 ${Math.round((downloaded / total) * 100)}%`
              : '下载中…',
            { id },
          );
          break;
        case 'Finished':
          toast.loading('正在安装…', { id });
          break;
      }
    });
    toast.success('更新完成，正在重启…', { id });
    await relaunch();
  } catch (e) {
    toast.error(`更新失败：${e}`, { id });
    installing = false;
  }
}
