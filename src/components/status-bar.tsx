import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { RefreshCw } from 'lucide-react';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip';
import { runUpdateCheck } from '@/lib/updater';
import { cn } from '@/lib/utils';

export interface GlobalTask {
  kind: 'compute' | 'upload';
  done: number;
  total: number;
  current?: string;
}

interface StatusBarProps {
  folderCount: number;
  totalFiles: number;
  pendingUpload: number;
  endpointConfigured: boolean;
  task: GlobalTask | null;
}

/**
 * 底部状态栏：常驻、固定高度（无纵向位移）。
 * 左侧聚合状态；右侧为全局任务进度（算码/上传），空闲时显示就绪。
 * 任务区计数固定宽度，进度条 flex-1 宽度恒定 → 不抖。
 */
export function StatusBar({
  folderCount,
  totalFiles,
  pendingUpload,
  endpointConfigured,
  task,
}: StatusBarProps) {
  const pct = task && task.total > 0 ? Math.round((task.done / task.total) * 100) : 0;

  const [version, setVersion] = useState('');
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(''));
  }, []);

  const checkUpdate = async () => {
    setChecking(true);
    try {
      await runUpdateCheck(false);
    } finally {
      setChecking(false);
    }
  };

  return (
    <footer className="flex h-8 shrink-0 items-center gap-3 border-t bg-background px-3 text-xs text-muted-foreground">
      {/* 左：聚合状态 */}
      <span className="tabular-nums">{folderCount} 文件夹</span>
      <span className="text-border">·</span>
      <span className="tabular-nums">{totalFiles} 文件</span>
      {pendingUpload > 0 && (
        <Badge variant="warning" className="tabular-nums">
          待传 {pendingUpload}
        </Badge>
      )}

      <div className="flex-1" />

      {/* 右：全局任务进度 / 就绪 */}
      {task ? (
        <div className="flex w-2/5 min-w-0 items-center gap-2">
          <span className="shrink-0 font-medium text-foreground">
            {task.kind === 'compute' ? '算码中' : '上传中'}
          </span>
          <Progress value={pct} className="h-1.5 min-w-0 flex-1" />
          <span className="shrink-0 whitespace-nowrap text-right tabular-nums">
            {task.done}/{task.total} ({pct}%)
          </span>
        </div>
      ) : (
        <span className="flex items-center gap-1.5">
          <span
            className={cn(
              'size-1.5 rounded-full',
              endpointConfigured ? 'bg-green-500' : 'bg-muted-foreground/40'
            )}
          />
          {endpointConfigured ? '已配置上传地址 · 就绪' : '未配置上传地址'}
        </span>
      )}

      <span className="text-border">·</span>
      {/* 版本号 + 检查更新（点击检查） */}
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            onClick={checkUpdate}
            disabled={checking}
            className="flex items-center gap-1 rounded px-1.5 py-0.5 tabular-nums transition-colors hover:bg-accent hover:text-foreground disabled:opacity-60"
          >
            v{version || '…'}
            <RefreshCw className={cn('size-3', checking && 'animate-spin')} />
          </button>
        </TooltipTrigger>
        <TooltipContent>检查更新</TooltipContent>
      </Tooltip>
    </footer>
  );
}
