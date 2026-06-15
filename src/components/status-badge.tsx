import {
  Circle,
  CircleCheck,
  TriangleAlert,
  CloudUpload,
  CloudCheck,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import type { FileStatus, UploadStatus } from '@/lib/tauri';

/** 算码状态徽章（详情面板用） */
export function ComputeBadge({ status }: { status: FileStatus | null }) {
  if (status == null) return null;
  if (status === 'uncomputed')
    return (
      <Badge variant="secondary">
        <Circle /> 未算
      </Badge>
    );
  if (status === 'needs_recompute')
    return (
      <Badge variant="warning">
        <TriangleAlert /> 需重算
      </Badge>
    );
  return (
    <Badge variant="success">
      <CircleCheck /> 已算
    </Badge>
  );
}

/** 上传状态徽章 */
export function UploadBadge({ status }: { status: UploadStatus | null }) {
  if (status == null) return null;
  if (status === 'pending')
    return (
      <Badge variant="warning">
        <CloudUpload /> 待传
      </Badge>
    );
  return (
    <Badge variant="success">
      <CloudCheck /> 已传
    </Badge>
  );
}

/**
 * 文件树行用：把算码+上传合并成单个徽章，密集行更干净。
 * 未算 / 需重算 / 待传 / 已传（已算且已传）。
 */
export function FileStatusBadge({
  compute,
  upload,
}: {
  compute: FileStatus | null;
  upload: UploadStatus | null;
}) {
  if (compute == null) return null;
  if (compute === 'uncomputed')
    return (
      <Badge variant="secondary">
        <Circle /> 未算
      </Badge>
    );
  if (compute === 'needs_recompute')
    return (
      <Badge variant="warning">
        <TriangleAlert /> 需重算
      </Badge>
    );
  if (upload === 'uploaded')
    return (
      <Badge variant="success">
        <CloudCheck /> 已传
      </Badge>
    );
  return (
    <Badge variant="warning">
      <CloudUpload /> 待传
    </Badge>
  );
}
