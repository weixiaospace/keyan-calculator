import { useCallback, useEffect, useRef, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { SettingsBar } from '@/components/settings-bar';
import { FolderList } from '@/components/folder-list';
import { FileDetail } from '@/components/file-detail';
import {
  api,
  onComputeProgress,
  onUploadProgress,
  type AppConfig,
  type ComputeProgress,
  type FileDetail as FileDetailData,
  type Folder,
  type ScanResult,
  type TreeNode,
  type UploadProgress,
} from '@/lib/tauri';
import { StatusBar, type GlobalTask } from '@/components/status-bar';
import { TooltipProvider } from '@/components/ui/tooltip';
import { Toaster } from '@/components/ui/sonner';
import { toast } from 'sonner';

interface SelectedFile {
  folderId: string;
  node: TreeNode;
}

export default function App() {
  const [folders, setFolders] = useState<Folder[]>([]);
  const [scans, setScans] = useState<Record<string, ScanResult | undefined>>({});
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [forceRecompute, setForceRecompute] = useState(false);

  const [expandedFolderId, setExpandedFolderId] = useState<string | null>(null);
  const [selected, setSelected] = useState<SelectedFile | null>(null);
  const [detail, setDetail] = useState<FileDetailData | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const [adding, setAdding] = useState(false);
  const [scanningFolderId, setScanningFolderId] = useState<string | null>(null);
  const [recomputing, setRecomputing] = useState(false);

  const [computeProgress, setComputeProgress] = useState<
    Record<string, ComputeProgress | undefined>
  >({});
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);

  // 用于在事件回调里拿到最新选中项
  const selectedRef = useRef<SelectedFile | null>(null);
  selectedRef.current = selected;

  // ---- 辅助 ----------------------------------------------------------------

  const scanFolder = useCallback(async (folderId: string) => {
    setScanningFolderId(folderId);
    try {
      const result = await api.scanFolder(folderId);
      setScans((prev) => ({ ...prev, [folderId]: result }));
      return result;
    } finally {
      setScanningFolderId((cur) => (cur === folderId ? null : cur));
    }
  }, []);

  const loadDetail = useCallback(async (folderId: string, relPath: string) => {
    setDetailLoading(true);
    try {
      const d = await api.getFileDetail(folderId, relPath);
      setDetail(d);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  // ---- 初始化 --------------------------------------------------------------

  useEffect(() => {
    (async () => {
      try {
        const [list, cfg] = await Promise.all([api.listFolders(), api.getConfig()]);
        setFolders(list);
        setConfig(cfg);
        if (list.length > 0) setExpandedFolderId(list[0].id);
        await Promise.all(list.map((f) => scanFolder(f.id)));
      } catch (err) {
        console.error('初始化失败', err);
      }
    })();
  }, [scanFolder]);

  // ---- 事件监听 ------------------------------------------------------------

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      onComputeProgress((p) => {
        setComputeProgress((prev) => ({ ...prev, [p.folder_id]: p }));
      })
    );
    unlisteners.push(
      onUploadProgress((p) => {
        setUploadProgress(p);
      })
    );

    return () => {
      unlisteners.forEach((u) => u.then((fn) => fn()).catch(() => {}));
    };
  }, []);

  // ---- 操作 ----------------------------------------------------------------

  const handleAddFolder = async () => {
    setAdding(true);
    try {
      const picked = await open({ directory: true, multiple: false });
      if (typeof picked !== 'string') return;
      const folder = await api.addFolder(picked);
      const list = await api.listFolders();
      setFolders(list);
      setExpandedFolderId(folder.id);
      await scanFolder(folder.id);
    } catch (err) {
      toast.error('添加文件夹失败', { description: String(err) });
    } finally {
      setAdding(false);
    }
  };

  const handleToggleFolder = (folderId: string) => {
    setExpandedFolderId((cur) => (cur === folderId ? null : folderId));
    if (!scans[folderId]) void scanFolder(folderId);
  };

  const handleRemove = async (folderId: string) => {
    try {
      await api.removeFolder(folderId);
      setFolders((prev) => prev.filter((f) => f.id !== folderId));
      setScans((prev) => {
        const next = { ...prev };
        delete next[folderId];
        return next;
      });
      if (selected?.folderId === folderId) {
        setSelected(null);
        setDetail(null);
      }
    } catch (err) {
      toast.error('移除文件夹失败', { description: String(err) });
    }
  };

  const handleCompute = async (folderId: string) => {
    setComputeProgress((prev) => ({
      ...prev,
      [folderId]: { folder_id: folderId, done: 0, total: 0 },
    }));
    try {
      const result = await api.computeFolder(folderId, forceRecompute);
      setScans((prev) => ({ ...prev, [folderId]: result }));
      const cur = selectedRef.current;
      if (cur && cur.folderId === folderId && !cur.node.is_directory) {
        await loadDetail(folderId, cur.node.rel_path);
      }
    } catch (err) {
      toast.error('算码失败', { description: String(err) });
    } finally {
      setComputeProgress((prev) => {
        const next = { ...prev };
        delete next[folderId];
        return next;
      });
    }
  };

  const handleSelectNode = useCallback(
    (folderId: string, node: TreeNode) => {
      setSelected({ folderId, node });
      if (!node.is_directory) {
        void loadDetail(folderId, node.rel_path);
      } else {
        setDetail(null);
      }
    },
    [loadDetail]
  );

  const handleRecompute = async () => {
    if (!selected || selected.node.is_directory) return;
    setRecomputing(true);
    try {
      await api.computeFile(selected.folderId, selected.node.rel_path, true);
      await loadDetail(selected.folderId, selected.node.rel_path);
      await scanFolder(selected.folderId);
    } catch (err) {
      toast.error('重算失败', { description: String(err) });
    } finally {
      setRecomputing(false);
    }
  };

  const handleUpload = async () => {
    setUploading(true);
    setUploadProgress({ done: 0, total: 0 });
    try {
      const result = await api.uploadPending();
      // 刷新所有已加载文件夹以更新上传状态
      await Promise.all(
        Object.keys(scans).map((id) => scanFolder(id))
      );
      const cur = selectedRef.current;
      if (cur && !cur.node.is_directory) {
        await loadDetail(cur.folderId, cur.node.rel_path);
      }
      if (result.failed > 0) {
        toast.error(`上传 ${result.uploaded} 成功 · ${result.failed} 失败`, {
          description: result.errors[0],
        });
      } else if (result.uploaded > 0) {
        toast.success(`已上传 ${result.uploaded} 条存证`);
      } else {
        toast.info('没有待上传的存证');
      }
    } catch (err) {
      toast.error('上传失败', { description: String(err) });
    } finally {
      setUploading(false);
      setUploadProgress(null);
    }
  };

  const handleSaveEndpoint = async (endpoint: string | null) => {
    const next: AppConfig = { upload_endpoint: endpoint };
    try {
      await api.setConfig(next);
      setConfig(next);
      toast.success(endpoint ? '已保存上传地址' : '已清除上传地址');
    } catch (err) {
      toast.error('保存配置失败', { description: String(err) });
    }
  };

  // 待传总数（汇总所有已扫描文件夹）
  const pendingUploadCount = Object.values(scans).reduce(
    (sum, s) => sum + (s?.counts.pending_upload ?? 0),
    0
  );
  const totalFiles = Object.values(scans).reduce(
    (sum, s) => sum + (s?.counts.total_files ?? 0),
    0
  );

  const directoryNode =
    selected && selected.node.is_directory ? selected.node : null;

  // 全局任务：上传优先，其次任一进行中的算码
  const activeCompute = Object.values(computeProgress).find(
    (p): p is ComputeProgress => !!p
  );
  const globalTask: GlobalTask | null = uploading
    ? {
        kind: 'upload',
        done: uploadProgress?.done ?? 0,
        total: uploadProgress?.total ?? 0,
      }
    : activeCompute
      ? {
          kind: 'compute',
          done: activeCompute.done,
          total: activeCompute.total,
          current: activeCompute.current,
        }
      : null;

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
        <SettingsBar
        config={config}
        onSaveEndpoint={handleSaveEndpoint}
        forceRecompute={forceRecompute}
        onToggleForce={setForceRecompute}
        pendingUploadCount={pendingUploadCount}
        uploading={uploading}
        onUpload={handleUpload}
      />
      <div className="flex min-h-0 flex-1">
        <FolderList
          folders={folders}
          scans={scans}
          expandedFolderId={expandedFolderId}
          selected={selected}
          scanningFolderId={scanningFolderId}
          computeProgress={computeProgress}
          onAddFolder={handleAddFolder}
          adding={adding}
          onToggleFolder={handleToggleFolder}
          onScan={scanFolder}
          onCompute={handleCompute}
          onRemove={handleRemove}
          onSelectNode={handleSelectNode}
        />
        <main className="flex min-w-0 flex-1">
          <FileDetail
            loading={detailLoading}
            detail={directoryNode ? null : detail}
            directoryNode={directoryNode}
            recomputing={recomputing}
            onRecompute={handleRecompute}
          />
        </main>
      </div>
      <StatusBar
        folderCount={folders.length}
        totalFiles={totalFiles}
        pendingUpload={pendingUploadCount}
        endpointConfigured={!!config?.upload_endpoint}
        task={globalTask}
      />
      </div>
      <Toaster position="bottom-right" richColors />
    </TooltipProvider>
  );
}
