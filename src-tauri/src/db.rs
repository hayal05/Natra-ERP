// Native persistence contract for NATRA ERP.
// The production build should use this layer with a SQLite driver and keep the
// database under the Windows application-data directory, never beside binaries.

pub const DB_SCHEMA_VERSION: i32 = 1;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS employees (
  id TEXT PRIMARY KEY,
  employee_number TEXT NOT NULL UNIQUE,
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  email TEXT UNIQUE,
  phone TEXT,
  department TEXT,
  position TEXT,
  hire_date TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS attendance (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL,
  attendance_date TEXT NOT NULL,
  check_in_at TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'present',
  token_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  UNIQUE(employee_id, attendance_date)
);
CREATE TABLE IF NOT EXISTS sync_outbox (
  id TEXT PRIMARY KEY,
  operation TEXT NOT NULL,
  entity TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT
);
"#;

/// Returns the stable database filename. The Tauri application-data directory
/// should be resolved by the native runtime and supplied to the SQLite driver.
pub fn database_filename() -> &'static str { "natra-erp.sqlite3" }
