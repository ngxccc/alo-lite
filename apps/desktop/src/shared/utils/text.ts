const DIACRITIC_MAP: Record<string, string> = {
  à: "a",
  á: "a",
  ạ: "a",
  ả: "a",
  ã: "a",
  â: "a",
  ầ: "a",
  ấ: "a",
  ậ: "a",
  ẩ: "a",
  ẫ: "a",
  ă: "a",
  ằ: "a",
  ắ: "a",
  ặ: "a",
  ẳ: "a",
  ẵ: "a",
  è: "e",
  é: "e",
  ẹ: "e",
  ẻ: "e",
  ẽ: "e",
  ê: "e",
  ề: "e",
  ế: "e",
  ệ: "e",
  ể: "e",
  ễ: "e",
  ì: "i",
  í: "i",
  ị: "i",
  ỉ: "i",
  ĩ: "i",
  ò: "o",
  ó: "o",
  ọ: "o",
  ỏ: "o",
  õ: "o",
  ô: "o",
  ồ: "o",
  ố: "o",
  ộ: "o",
  ổ: "o",
  ỗ: "o",
  ơ: "o",
  ờ: "o",
  ớ: "o",
  ợ: "o",
  ở: "o",
  ỡ: "o",
  ù: "u",
  ú: "u",
  ụ: "u",
  ủ: "u",
  ũ: "u",
  ư: "u",
  ừ: "u",
  ứ: "u",
  ự: "u",
  ử: "u",
  ữ: "u",
  ỳ: "y",
  ý: "y",
  ỵ: "y",
  ỷ: "y",
  ỹ: "y",
  đ: "d",
};

export function foldVnSearch(input: string): string {
  let out = "";
  for (const ch of input.toLowerCase()) {
    out += DIACRITIC_MAP[ch] ?? ch;
  }
  return out;
}

export function matchesVnQuery(haystack: string, query: string): boolean {
  const qLow = query.trim().toLowerCase();
  if (!qLow) return true;
  const hLow = haystack.toLowerCase();
  if (hLow.includes(qLow)) return true;
  return foldVnSearch(hLow).includes(foldVnSearch(qLow));
}

export function normalizeTsMs(ts: number): number {
  return ts > 0 && ts < 10_000_000_000 ? ts * 1000 : ts;
}

export function formatClockNow(): string {
  return new Date().toLocaleTimeString("vi-VN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

export function formatDayDivider(ts: number): string {
  const ms = normalizeTsMs(ts);
  if (!ms) return "";
  const d = new Date(ms);
  const today = new Date();
  const isSameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();
  if (isSameDay(d, today)) return "Hôm nay";
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (isSameDay(d, yesterday)) return "Hôm qua";
  return d.toLocaleDateString("vi-VN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

export function dayKey(ts: number): string {
  const ms = normalizeTsMs(ts);
  if (!ms) return "";
  const d = new Date(ms);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

export function extractFirstUrl(text: string): string | null {
  const match = text.match(/https?:\/\/[^\s<>()]+/i);
  if (!match) return null;
  return match[0].replace(/[.,!?;:'")]+$/, "");
}

export function jxlToJpgUrl(url: string): string {
  return url.replace("/jxl/", "/jpg/").replace(/\.jxl(\?|$)/i, ".jpg$1");
}
export function avatarColor(name: string): string {
  const palette = [
    "#2563EB",
    "#059669",
    "#D97706",
    "#7C3AED",
    "#DB2777",
    "#0891B2",
    "#DC2626",
  ];
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) {
    hash = (hash * 31 + name.charCodeAt(i)) >>> 0;
  }
  return palette[hash % palette.length];
}

export function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "Z";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}
