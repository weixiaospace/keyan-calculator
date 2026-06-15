import { useEffect, useState } from 'react';
import { useTheme } from 'next-themes';
import { Settings, UploadCloud, Loader2, Sun, Moon, RefreshCw } from 'lucide-react';
import { runUpdateCheck } from '@/lib/updater';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from '@/components/ui/tooltip';
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from '@/components/ui/popover';
import type { AppConfig } from '@/lib/tauri';

interface SettingsBarProps {
  config: AppConfig | null;
  onSaveEndpoint: (endpoint: string | null) => void;
  forceRecompute: boolean;
  onToggleForce: (value: boolean) => void;
  pendingUploadCount: number;
  uploading: boolean;
  onUpload: () => void;
}

export function SettingsBar({
  config,
  onSaveEndpoint,
  forceRecompute,
  onToggleForce,
  pendingUploadCount,
  uploading,
  onUpload,
}: SettingsBarProps) {
  const { resolvedTheme, setTheme } = useTheme();
  const isDark = resolvedTheme === 'dark';

  const [endpointOpen, setEndpointOpen] = useState(false);
  const [endpointDraft, setEndpointDraft] = useState('');
  const [checkingUpdate, setCheckingUpdate] = useState(false);

  const checkUpdate = async () => {
    setCheckingUpdate(true);
    try {
      await runUpdateCheck(false);
    } finally {
      setCheckingUpdate(false);
    }
  };

  useEffect(() => {
    setEndpointDraft(config?.upload_endpoint ?? '');
  }, [config?.upload_endpoint]);

  const saveEndpoint = () => {
    const trimmed = endpointDraft.trim();
    onSaveEndpoint(trimmed.length > 0 ? trimmed : null);
    setEndpointOpen(false);
  };

  return (
    <header className="flex h-14 items-center gap-3 border-b bg-background px-4">
      {/* 品牌 */}
      <div className="flex items-center gap-2.5">
        <img src="/app-icon.svg" alt="算码工具" className="size-7" draggable={false} />
        <div className="leading-tight">
          <div className="text-sm font-semibold">算码工具</div>
          <div className="text-[11px] text-muted-foreground">SM3 · 时间存证</div>
        </div>
      </div>

      <div className="flex-1" />

      {/* 强制重算 */}
      <div className="flex items-center gap-2">
        <Switch
          id="force-recompute"
          checked={forceRecompute}
          onCheckedChange={onToggleForce}
        />
        <Label htmlFor="force-recompute" className="cursor-pointer text-sm">
          强制重算
        </Label>
      </div>

      {/* 亮暗切换 */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => setTheme(isDark ? 'light' : 'dark')}
          >
            {isDark ? <Sun className="size-4" /> : <Moon className="size-4" />}
          </Button>
        </TooltipTrigger>
        <TooltipContent>{isDark ? '切换到亮色' : '切换到暗色'}</TooltipContent>
      </Tooltip>

      {/* 检查更新 */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={checkUpdate}
            disabled={checkingUpdate}
          >
            <RefreshCw className={`size-4 ${checkingUpdate ? 'animate-spin' : ''}`} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>检查更新</TooltipContent>
      </Tooltip>

      {/* 上传地址设置 */}
      <Popover open={endpointOpen} onOpenChange={setEndpointOpen}>
        <PopoverTrigger asChild>
          <Button variant="outline" size="sm">
            <Settings className="size-4" />
            上传地址
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="flex w-96 flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="endpoint">上传地址 (Endpoint)</Label>
            <Input
              id="endpoint"
              value={endpointDraft}
              placeholder="https://example.com/upload"
              onChange={(e) => setEndpointDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') saveEndpoint();
              }}
            />
            <p className="text-xs text-muted-foreground">
              留空则清除。上传时把存证 POST 到此地址。
            </p>
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={() => setEndpointOpen(false)}>
              取消
            </Button>
            <Button size="sm" onClick={saveEndpoint}>
              保存
            </Button>
          </div>
        </PopoverContent>
      </Popover>

      {/* 批量上传 */}
      <Button
        size="sm"
        onClick={onUpload}
        disabled={uploading || pendingUploadCount === 0}
      >
        {uploading ? (
          <Loader2 className="size-4 animate-spin" />
        ) : (
          <UploadCloud className="size-4" />
        )}
        批量上传
        {pendingUploadCount > 0 && (
          <Badge variant="secondary" className="tabular-nums">
            {pendingUploadCount}
          </Badge>
        )}
      </Button>
    </header>
  );
}
