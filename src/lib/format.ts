/** 格式化文件大小（字节 -> KB/MB/GB） */
export function formatFileSize(bytes: number | null | undefined): string {
  if (bytes == null) return '-';
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(2)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

function pad(n: number, len = 2): string {
  return n.toString().padStart(len, '0');
}

/** 把 unix 秒格式化为可读日期（到秒） */
export function formatSeconds(sec: number | null | undefined): string {
  if (sec == null) return '-';
  return formatDate(new Date(sec * 1000), false);
}

/** 把 unix 毫秒格式化为可读日期（到毫秒） */
export function formatMillis(ms: number | null | undefined): string {
  if (ms == null) return '-';
  return formatDate(new Date(ms), true);
}

function formatDate(d: Date, withMillis: boolean): string {
  const base = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  return withMillis ? `${base}.${pad(d.getMilliseconds(), 3)}` : base;
}

/** 把 64 位 hex 派生码每 8 字符插入空格分段 */
export function chunkCode(code: string, size = 8): string {
  const out: string[] = [];
  for (let i = 0; i < code.length; i += size) {
    out.push(code.slice(i, i + size));
  }
  return out.join(' ');
}

/** 缩写一个长 hex（前 8 … 后 8） */
export function abbreviate(code: string, head = 8, tail = 8): string {
  if (code.length <= head + tail + 1) return code;
  return `${code.slice(0, head)}…${code.slice(-tail)}`;
}
