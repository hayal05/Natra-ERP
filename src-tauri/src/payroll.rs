use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SalaryRecord {
    pub id: String,
    pub employee_id: String,
    pub pay_period: String,
    pub base_salary: f64,
    pub allowances: f64,
    pub deductions: f64,
    pub net_salary: f64,
    pub status: String,
}

pub fn migrate(c: &Connection) -> Result<(), rusqlite::Error> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS salary_records(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL REFERENCES employees(id),pay_period TEXT NOT NULL,base_salary REAL NOT NULL DEFAULT 0,allowances REAL NOT NULL DEFAULT 0,deductions REAL NOT NULL DEFAULT 0,net_salary REAL NOT NULL DEFAULT 0,status TEXT NOT NULL DEFAULT 'draft',created_at TEXT NOT NULL,updated_at TEXT NOT NULL,UNIQUE(employee_id,pay_period)); CREATE INDEX IF NOT EXISTS idx_salary_period ON salary_records(pay_period); CREATE INDEX IF NOT EXISTS idx_salary_employee ON salary_records(employee_id);")
}

pub fn create(c: &mut Connection, salary: &SalaryRecord, now: &str) -> Result<(), rusqlite::Error> {
    let tx = c.transaction()?;
    tx.execute("INSERT INTO salary_records(id,employee_id,pay_period,base_salary,allowances,deductions,net_salary,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)", params![salary.id,salary.employee_id,salary.pay_period,salary.base_salary,salary.allowances,salary.deductions,salary.net_salary,salary.status,now,now])?;
    queue(&tx, salary, now)?;
    tx.commit()
}

fn queue(tx: &rusqlite::Transaction<'_>, salary: &SalaryRecord, now: &str) -> Result<(), rusqlite::Error> {
    let payload = serde_json::to_string(salary).unwrap_or_default();
    tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','salary',?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL", params![format!("sync-salary-{}",salary.id),salary.id,payload,now])?;
    Ok(())
}
