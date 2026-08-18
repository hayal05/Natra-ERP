import { createClient } from '@libsql/client';
import crypto from 'node:crypto';

const db = createClient({ url: process.env.TURSO_DATABASE_URL, authToken: process.env.TURSO_AUTH_TOKEN });

export default async function handler(req, res) {
  if (req.method !== 'POST') return res.status(405).json({ code: 'METHOD_NOT_ALLOWED' });
  const employeeId = typeof req.body?.employeeId === 'string' ? req.body.employeeId.trim() : '';
  if (!employeeId) return res.status(400).json({ code: 'INVALID_EMPLOYEE' });

  // Authentication will supply the employee identity in the next auth step.
  // Until then this endpoint is intentionally marked development-only.
  if (process.env.NODE_ENV === 'production' && process.env.NATRA_ATTENDANCE_TOKEN_ISSUING !== 'enabled') {
    return res.status(503).json({ code: 'TOKEN_ISSUING_NOT_ENABLED' });
  }

  const employee = await db.execute({ sql: 'SELECT id,status FROM employees WHERE id=? LIMIT 1', args: [employeeId] });
  if (!employee.rows.length) return res.status(404).json({ code: 'EMPLOYEE_NOT_FOUND' });
  if (String(employee.rows[0].status) !== 'active') return res.status(403).json({ code: 'EMPLOYEE_INACTIVE' });

  const token = crypto.randomBytes(32).toString('base64url');
  const now = new Date();
  const expires = new Date(now.getTime() + 60_000).toISOString();
  await db.execute({ sql: 'INSERT INTO attendance_tokens(id,employee_id,expires_at,created_at) VALUES(?,?,?,?)', args: [token, employeeId, expires, now.toISOString()] });
  return res.status(201).json({ token, expiresAt: expires });
}
