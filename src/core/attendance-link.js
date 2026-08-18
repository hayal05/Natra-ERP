const ATTENDANCE_BASE = 'https://attendance.natra-erp.com/a/';

export function createAttendanceUrl(token) {
  if (!token?.id) throw new Error('Invalid attendance token');
  return `${ATTENDANCE_BASE}${encodeURIComponent(token.id)}`;
}

export function parseAttendanceTokenFromUrl(value) {
  try {
    const url = new URL(value);
    const prefix = new URL(ATTENDANCE_BASE);
    if (url.origin !== prefix.origin || !url.pathname.startsWith(prefix.pathname)) return null;
    const token = decodeURIComponent(url.pathname.slice(prefix.pathname.length)).trim();
    return token || null;
  } catch { return null; }
}
