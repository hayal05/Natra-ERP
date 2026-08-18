CREATE TABLE IF NOT EXISTS employees (
  id TEXT PRIMARY KEY,
  employee_number TEXT NOT NULL UNIQUE,
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  email TEXT,
  department TEXT,
  position TEXT,
  status TEXT NOT NULL DEFAULT 'active'
);
CREATE TABLE IF NOT EXISTS attendance_tokens (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees(id),
  expires_at TEXT NOT NULL,
  used_at TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attendance_tokens_employee ON attendance_tokens(employee_id);
CREATE INDEX IF NOT EXISTS idx_attendance_tokens_expiry ON attendance_tokens(expires_at);
CREATE TABLE IF NOT EXISTS attendance (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees(id),
  attendance_date TEXT NOT NULL,
  check_in_at TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'present',
  token_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  UNIQUE(employee_id, attendance_date)
);
CREATE INDEX IF NOT EXISTS idx_attendance_date ON attendance(attendance_date);
