import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ---------------------------------------------------------------------------
// 状态枚举
// ---------------------------------------------------------------------------

/** 未算 / 需重算 / 已算 */
export type FileStatus = 'uncomputed' | 'needs_recompute' | 'computed';
/** 待传 / 已传 */
export type UploadStatus = 'pending' | 'uploaded';

// ---------------------------------------------------------------------------
// 数据结构
// ---------------------------------------------------------------------------

export interface Folder {
  id: string;
  root_path: string;
  name: string;
  /** unix ms */
  last_scan_at: number | null;
}

export interface TreeNode {
  name: string;
  /** 相对文件夹根的路径 */
  rel_path: string;
  is_directory: boolean;
  /** 目录为 null */
  file_size: number | null;
  /** 目录为 null */
  compute_status: FileStatus | null;
  /** 目录 / 未算为 null */
  upload_status: UploadStatus | null;
  children: TreeNode[];
}

export interface Attestation {
  id: string;
  /** 完整路径字符串（root + rel） */
  path: string;
  file_name: string;
  file_size: number;
  /** unix 秒 */
  created_time: number;
  /** unix 秒 */
  modified_time: number;
  /** 64 hex */
  sm3: string;
  /** unix 毫秒 */
  calc_ts: number;
  /** 64 hex */
  derived_code: string;
  /** "local" */
  time_source: string;
  /** unix 毫秒，null=待传 */
  uploaded_at: number | null;
}

export interface FileDetail {
  rel_path: string;
  full_path: string;
  file_name: string;
  file_size: number | null;
  /** unix 秒 */
  created_time: number | null;
  /** unix 秒 */
  modified_time: number | null;
  /** 该路径最新存证（当前码） */
  latest: Attestation | null;
  /** 该路径全部存证，按 calc_ts 倒序 */
  history: Attestation[];
  exists_on_disk: boolean;
}

export interface ScanCounts {
  total_files: number;
  uncomputed: number;
  needs_recompute: number;
  computed: number;
  pending_upload: number;
}

export interface ScanResult {
  folder_id: string;
  tree: TreeNode[];
  counts: ScanCounts;
}

export interface ComputeResult {
  computed: number;
  skipped: number;
}

export interface UploadResult {
  uploaded: number;
  failed: number;
  errors: string[];
}

export interface AppConfig {
  upload_endpoint: string | null;
}

// ---------------------------------------------------------------------------
// 事件 payload
// ---------------------------------------------------------------------------

export interface ComputeProgress {
  folder_id: string;
  done: number;
  total: number;
  current?: string;
}

export interface UploadProgress {
  done: number;
  total: number;
}

// ---------------------------------------------------------------------------
// 命令封装
// ---------------------------------------------------------------------------

function listFolders(): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders');
}

function addFolder(path: string): Promise<Folder> {
  return invoke<Folder>('add_folder', { path });
}

function removeFolder(folderId: string): Promise<void> {
  return invoke<void>('remove_folder', { folderId });
}

function scanFolder(folderId: string): Promise<ScanResult> {
  return invoke<ScanResult>('scan_folder', { folderId });
}

function computeFolder(folderId: string, force: boolean): Promise<ScanResult> {
  return invoke<ScanResult>('compute_folder', { folderId, force });
}

function computeFile(
  folderId: string,
  relPath: string,
  force: boolean
): Promise<ComputeResult> {
  return invoke<ComputeResult>('compute_file', { folderId, relPath, force });
}

function getFileDetail(folderId: string, relPath: string): Promise<FileDetail> {
  return invoke<FileDetail>('get_file_detail', { folderId, relPath });
}

function uploadPending(): Promise<UploadResult> {
  return invoke<UploadResult>('upload_pending');
}

function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

function setConfig(config: AppConfig): Promise<void> {
  return invoke<void>('set_config', { config });
}

export const api = {
  listFolders,
  addFolder,
  removeFolder,
  scanFolder,
  computeFolder,
  computeFile,
  getFileDetail,
  uploadPending,
  getConfig,
  setConfig,
};

// ---------------------------------------------------------------------------
// 事件监听封装
// ---------------------------------------------------------------------------

export function onComputeProgress(
  cb: (payload: ComputeProgress) => void
): Promise<UnlistenFn> {
  return listen<ComputeProgress>('compute-progress', (e) => cb(e.payload));
}

export function onUploadProgress(
  cb: (payload: UploadProgress) => void
): Promise<UnlistenFn> {
  return listen<UploadProgress>('upload-progress', (e) => cb(e.payload));
}
