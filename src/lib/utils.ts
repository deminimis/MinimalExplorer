export function formatSize(bytes: number, isDir: boolean) {
  if (isDir) return "";
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

const dateFormatter = new Intl.DateTimeFormat(undefined, { 
  year: 'numeric', month: '2-digit', day: '2-digit', 
  hour: 'numeric', minute: '2-digit', hour12: true 
});
export const formatDate = (timestamp: number) => timestamp === 0 ? "" : dateFormatter.format(new Date(timestamp * 1000));

export function isImage(name: string) {
  return /\.(jpg|jpeg|png|gif|webp|bmp)$/i.test(name);
}

export function getSystemIconSrc(path: string, is_dir: boolean): string {
  // Send the absolute path URL-encoded so the backend can fetch the precise icon
  return `http://icon.localhost/${encodeURIComponent(path)}?is_dir=${is_dir}`;
}