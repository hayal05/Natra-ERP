import { createClient } from '@libsql/client';

const db = createClient({
  url: process.env.TURSO_DATABASE_URL,
  authToken: process.env.TURSO_AUTH_TOKEN,
});

export default async function handler(req, res) {
  if (req.method !== 'POST') return res.status(405).json({ code: 'METHOD_NOT_ALLOWED' });
  const token = typeof req.body?.token === 'string' ? req.body.token.trim() : '';
  if (!token || token.length > 256) return res.status(400).json({ code: 'INVALID_TOKEN' });

  try {
    const now = new Date();
    const nowIso = now.toISOString();
    const today = nowIso.slice(0, 10);
    const result = await db.batch([
      { sql: 'SELECT t.id,t.employee_id,t.expires_at,t.used_at,e.first_name,e.last_name,e.department,e.status FROM attendance_tokens t JOIN employees e ON e.id=t.employee_id WHERE t.id=? LIMIT 1', args: [token] },
    ], 'read');
    const row = result[0]?.rows?.[0];
    if (!row) return res.status(404).json({ code: 'INVALID_TOKEN' });
    if (row.used_at) return res.status(409).json({ code: 'TOKEN_ALREADY_USED' });
    if (new Date(String(row.expires_at)) <= now) return res.status(410).json({ code: 'EXPIRED_TOKEN' });
    if (String(row.status) !== 'active') return res.status(403).json({ code: 'EMPLOYEE_INACTIVE' });

    const attendanceId = crypto.randomUUID();
    const employeeId = String(row.employee_id);
    const lock = await db.batch([
      { sql: 'UPDATE attendance_tokens SET used_at=? WHERE id=? AND used_at IS NULL AND expires_at>?', args: [nowIso, token, nowIso] },
      { sql: 'SELECT id FROM attendance WHERE employee_id=? AND attendance_date=? LIMIT 1', args: [employeeId, today] },
    ], 'write');
    if (Number(lock[0]?.rowsAffected || 0) !== 1) return res.status(409).json({ code: 'TOKEN_ALREADY_USED' });
    if (lock[1]?.rows?.length) return res.status(409).json({ code: 'ALREADY_ATTENDED_TODAY' });

    try {
      await db.batch([
        { sql: 'INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?,?,?,? ,\'present\',?,?)', args: [attendanceId, employeeId, today, nowIso, token, nowIso] },
      ], 'write');
    } catch (error) {
      await db.execute({ sql: 'UPDATE attendance_tokens SET used_at=NULL WHERE id=? AND used_at=?', args: [token, nowIso] });
      throw error;
    }

    return res.status(200).json({ recorded: true, attendanceId, employee: { id: employeeId, displayName: `${row.first_name} ${row.last_name}` }, checkInAt: nowIso });
  } catch (error) {
    console.error('attendance verification failed', error);
    return res.status(500).json({ code: 'VERIFICATION_FAILED' });
  }
}
