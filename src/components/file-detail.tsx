import type { ReactNode } from 'react';
import {
  FileText,
  RefreshCw,
  FileSearch,
  FolderOpen,
  Loader2,
  History,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardAction,
  CardContent,
} from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import {
  Empty,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  EmptyDescription,
} from '@/components/ui/empty';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from '@/components/ui/tooltip';
import { CopyButton } from '@/components/copy-button';
import { ComputeBadge, UploadBadge } from '@/components/status-badge';
import type { FileDetail as FileDetailData, TreeNode } from '@/lib/tauri';
import {
  abbreviate,
  chunkCode,
  formatFileSize,
  formatMillis,
  formatSeconds,
} from '@/lib/format';

interface FileDetailProps {
  loading: boolean;
  detail: FileDetailData | null;
  directoryNode: TreeNode | null;
  recomputing: boolean;
  onRecompute: () => void;
}

function timeSourceLabel(source: string): string {
  return source === 'local' ? '本机时钟' : source;
}

export function FileDetail({
  loading,
  detail,
  directoryNode,
  recomputing,
  onRecompute,
}: FileDetailProps) {
  if (directoryNode) return <DirectorySummary node={directoryNode} />;
  if (loading) return <DetailSkeleton />;
  if (!detail) {
    return (
      <Empty className="h-full flex-1">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <FileSearch />
          </EmptyMedia>
          <EmptyTitle>未选择文件</EmptyTitle>
          <EmptyDescription>在左侧选择一个文件，查看它的存证详情。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }

  const { latest } = detail;

  return (
    <section className="flex h-full flex-1 flex-col overflow-hidden">
      {/* header */}
      <div className="flex items-start gap-3 border-b p-5">
        <FileText className="mt-0.5 size-5 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h2 className="truncate text-lg font-semibold">{detail.file_name}</h2>
            {latest ? (
              <UploadBadge status={latest.uploaded_at == null ? 'pending' : 'uploaded'} />
            ) : (
              <ComputeBadge status="uncomputed" />
            )}
          </div>
          <p className="truncate text-xs text-muted-foreground">{detail.full_path}</p>
          {!detail.exists_on_disk && (
            <p className="mt-1 text-xs text-destructive">该文件已不在磁盘上</p>
          )}
        </div>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="outline" size="sm" onClick={onRecompute} disabled={recomputing}>
              {recomputing ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <RefreshCw className="size-4" />
              )}
              重算
            </Button>
          </TooltipTrigger>
          <TooltipContent>忽略缓存，重新算码并追加一条新存证</TooltipContent>
        </Tooltip>
      </div>

      {/* body */}
      <div className="min-h-0 flex-1 overflow-y-auto p-5">
        {latest ? (
          <Tabs defaultValue="detail" className="gap-4">
            <TabsList>
              <TabsTrigger value="detail">详情</TabsTrigger>
              <TabsTrigger value="history">历史 {detail.history.length}</TabsTrigger>
            </TabsList>

            <TabsContent value="detail" className="flex flex-col gap-4">
              {/* 时间存证码 — hero */}
              <Card>
                <CardHeader>
                  <CardTitle>时间存证码</CardTitle>
                  <CardDescription>
                    SM3(内容 · 创建 · 修改 · 算码时刻) · 不可变
                  </CardDescription>
                  <CardAction>
                    <CopyButton value={latest.derived_code} />
                  </CardAction>
                </CardHeader>
                <CardContent>
                  <code className="block rounded-md bg-muted px-3 py-2.5 font-mono text-sm leading-relaxed tracking-wider break-all">
                    {chunkCode(latest.derived_code)}
                  </code>
                  <p className="mt-2 font-mono text-xs text-muted-foreground">
                    封存于 {formatMillis(latest.calc_ts)} · {timeSourceLabel(latest.time_source)}
                  </p>
                </CardContent>
              </Card>

              {/* 字段 */}
              <Card>
                <CardContent className="flex flex-col gap-3">
                  <Row label="SM3 内容指纹">
                    <div className="flex min-w-0 items-center gap-1">
                      <code className="truncate font-mono text-xs text-muted-foreground">
                        {latest.sm3}
                      </code>
                      <CopyButton value={latest.sm3} className="shrink-0" />
                    </div>
                  </Row>
                  <Row label="大小">{formatFileSize(detail.file_size)}</Row>
                  <Row label="创建时间">{formatSeconds(detail.created_time)}</Row>
                  <Row label="修改时间">{formatSeconds(detail.modified_time)}</Row>
                  <Row label="上传状态">
                    <span className="flex items-center gap-2">
                      <UploadBadge status={latest.uploaded_at == null ? 'pending' : 'uploaded'} />
                      {latest.uploaded_at != null && (
                        <span className="font-mono text-xs text-muted-foreground">
                          {formatMillis(latest.uploaded_at)}
                        </span>
                      )}
                    </span>
                  </Row>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="history">
              <HistoryList detail={detail} />
            </TabsContent>
          </Tabs>
        ) : (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <RefreshCw />
              </EmptyMedia>
              <EmptyTitle>尚未算码</EmptyTitle>
              <EmptyDescription>
                点击右上角「重算」，为该文件生成时间存证码。
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </div>
    </section>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <div className="min-w-0 text-right">{children}</div>
    </div>
  );
}

function HistoryList({ detail }: { detail: FileDetailData }) {
  if (detail.history.length === 0) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <History />
          </EmptyMedia>
          <EmptyTitle>暂无历史存证</EmptyTitle>
        </EmptyHeader>
      </Empty>
    );
  }
  return (
    <div className="flex flex-col gap-1.5">
      {detail.history.map((att) => (
        <div
          key={att.id}
          className="flex items-center gap-3 rounded-md border px-3 py-2 text-xs"
        >
          <span className="font-mono tabular-nums text-muted-foreground">
            {formatMillis(att.calc_ts)}
          </span>
          <code className="font-mono">{abbreviate(att.derived_code)}</code>
          <span className="ml-auto">
            <UploadBadge status={att.uploaded_at == null ? 'pending' : 'uploaded'} />
          </span>
        </div>
      ))}
    </div>
  );
}

function DirectorySummary({ node }: { node: TreeNode }) {
  const acc = { files: 0, uncomputed: 0, needsRecompute: 0, computed: 0, pending: 0 };
  const walk = (n: TreeNode) => {
    if (!n.is_directory) {
      acc.files += 1;
      if (n.compute_status === 'uncomputed') acc.uncomputed += 1;
      else if (n.compute_status === 'needs_recompute') acc.needsRecompute += 1;
      else if (n.compute_status === 'computed') {
        acc.computed += 1;
        if (n.upload_status === 'pending') acc.pending += 1;
      }
    }
    n.children.forEach(walk);
  };
  node.children.forEach(walk);

  const stats = [
    { label: '文件数', value: acc.files },
    { label: '已算', value: acc.computed },
    { label: '未算', value: acc.uncomputed },
    { label: '需重算', value: acc.needsRecompute },
    { label: '待传', value: acc.pending },
  ];

  return (
    <section className="flex h-full flex-1 flex-col overflow-y-auto p-5">
      <div className="flex items-center gap-2">
        <FolderOpen className="size-5 text-sky-500" />
        <h2 className="truncate text-lg font-semibold">{node.name}</h2>
      </div>
      <p className="mt-0.5 truncate text-xs text-muted-foreground">
        {node.rel_path || '（根目录）'}
      </p>
      <div className="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-3">
        {stats.map((s) => (
          <div key={s.label} className="rounded-lg border p-4">
            <div className="text-xs text-muted-foreground">{s.label}</div>
            <div className="mt-1 text-2xl font-semibold tabular-nums">{s.value}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function DetailSkeleton() {
  return (
    <section className="flex h-full flex-1 flex-col gap-4 p-5">
      <div className="flex items-center gap-3">
        <Skeleton className="size-5 rounded-md" />
        <Skeleton className="h-5 w-40" />
      </div>
      <Skeleton className="h-3 w-72" />
      <Skeleton className="mt-2 h-28 w-full rounded-xl" />
      <Skeleton className="h-40 w-full rounded-xl" />
    </section>
  );
}
