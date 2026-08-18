/**
 * Attendance domain rules.
 * Token generation is intentionally UI-agnostic so it can later be backed by
 * a secure API/Turso service without rewriting the desktop UI.
 */

const TOKEN_TTL_MS = 60 * 1000;

export function createAttendanceToken(employeeId, now = new Date()) {
  if (!employeeId) throw new Error('Employee identity is required.');

  const id = crypto.randomUUID();
  const expiresAt = new Date(now.getTime() + TOKEN_TTL_MS);
  const nonce = crypto.randomUUID().replaceAll('-', '');

  return {
    id,
    employeeId,
    nonce,
    expiresAt: expiresAt.toISOString(),
    createdAt: now.toISOString()
  };
}

export function isTokenValid(token, now = new Date()) {
  return Boolean(token && !token.consumedAt && new Date(token.expiresAt).getTime() > now.getTime());
}

export function consumeToken(token, now = new Date()) {
  if (!isTokenValid(token, now)) {
    throw new Error('This attendance QR has expired or has already been used.');
  }

  return { ...token, consumedAt: now.toISOString() };
}

export function createAttendanceRecord(employee, token, now = new Date()) {
  const date = now.toISOString().slice(0, 10);
  return {
    id: crypto.randomUUID(),
    employeeId: employee.id,
    employeeNumber: employee.employeeNumber,
    employeeName: `${employee.firstName} ${employee.lastName}`.trim(),
    department: employee.department ?? '',
    attendanceDate: date,
    checkInAt: now.toISOString(),
    status: 'present',
    tokenId: token.id,
    createdAt: now.toISOString()
  };
}
