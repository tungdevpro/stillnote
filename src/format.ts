const cache = new Map<string, Intl.DateTimeFormat>();

function fmt(locale: string, options: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const key = locale + JSON.stringify(options);
  let f = cache.get(key);
  if (!f) {
    f = new Intl.DateTimeFormat(locale, options);
    cache.set(key, f);
  }
  return f;
}

function startOfDay(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** Short date for the note list: "2:05 PM", "Yesterday", "Sep 12", "Feb 3, 2025". */
export function shortDate(ms: number, locale: string, yesterday: string): string {
  const d = new Date(ms);
  const now = new Date();
  const days = Math.round((startOfDay(now) - startOfDay(d)) / 86_400_000);
  if (days === 0) return fmt(locale, { hour: "numeric", minute: "2-digit" }).format(d);
  if (days === 1) return yesterday;
  if (d.getFullYear() === now.getFullYear()) return fmt(locale, { day: "numeric", month: "short" }).format(d);
  return fmt(locale, { day: "numeric", month: "short", year: "numeric" }).format(d);
}

export function longDate(ms: number, locale: string): string {
  return fmt(locale, {
    day: "numeric",
    month: "long",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(ms));
}
