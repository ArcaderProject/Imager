export const formatBytes = (bytes) => {
    if (!bytes || bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = bytes;
    let i = 0;
    while (v >= 1000 && i < units.length - 1) {
        v /= 1000;
        i += 1;
    }
    return `${i >= 3 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
};

export const formatRate = (bytesPerSec) => {
    if (!bytesPerSec || bytesPerSec <= 0) return "-";
    return `${formatBytes(bytesPerSec)}/s`;
};

export const formatEta = (remainingBytes, bytesPerSec) => {
    if (!bytesPerSec || bytesPerSec <= 0 || remainingBytes <= 0) return "-";
    const secs = Math.round(remainingBytes / bytesPerSec);
    if (secs < 60) return `${secs}s`;
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}m ${s.toString().padStart(2, "0")}s`;
};
