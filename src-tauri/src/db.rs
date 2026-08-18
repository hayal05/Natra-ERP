use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};
use thiserror::Error;

pub const DB_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Error)]
pub enum DbError { #[error("database error: {0}")] Sql(#[from] rusqlite::Error), #[error("filesystem error: {0}")] Io(#[from] std::io::Error) }

#[derive(Clone)]
pub struct Database { path: PathBuf }

#[derive(Debug, Serialize)]
pub struct Employee { pub id:String,pub employee_number:String,pub first_name:String,pub last_name:String,pub email:Option<String>,pub phone:Option<String>,pub department:Option<String>,pub position:Option<String>,pub hire_date:Option<String>,pub status:String }

impl Database {
  pub fn open(app_data_dir:&Path)->Result<Self,DbError>{ fs::create_dir_all(app_data_dir)?; let db=Self{path:app_data_dir.join("natra-erp.sqlite3")}; db.migrate()?; Ok(db) }
  fn connect(&self)->Result<Connection,DbError>{ let c=Connection::open(&self.path)?; c.pragma_update(None,"foreign_keys","ON")?; c.pragma_update(None,"journal_mode","WAL")?; Ok(c) }
  fn migrate(&self)->Result<(),DbError>{ let c=self.connect()?; c.execute_batch(r#"
    CREATE TABLE IF NOT EXISTS schema_meta(version INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS employees(id TEXT PRIMARY KEY,employee_number TEXT NOT NULL UNIQUE,first_name TEXT NOT NULL,last_name TEXT NOT NULL,email TEXT UNIQUE,phone TEXT,department TEXT,position TEXT,hire_date TEXT,status TEXT NOT NULL DEFAULT 'active',created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS attendance(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL REFERENCES employees(id),attendance_date TEXT NOT NULL,check_in_at TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'present',token_id TEXT NOT NULL UNIQUE,created_at TEXT NOT NULL,UNIQUE(employee_id,attendance_date));
    CREATE TABLE IF NOT EXISTS sync_outbox(id TEXT PRIMARY KEY,operation TEXT NOT NULL,entity TEXT NOT NULL,entity_id TEXT NOT NULL,payload TEXT NOT NULL,created_at TEXT NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,last_error TEXT);
    CREATE INDEX IF NOT EXISTS idx_attendance_date ON attendance(attendance_date);
  "#)?; let current:Option<i32>=c.query_row("SELECT version FROM schema_meta LIMIT 1",[],|r|r.get(0)).optional()?; if current.is_none(){c.execute("INSERT INTO schema_meta(version) VALUES (?)",[DB_SCHEMA_VERSION])?;} Ok(()) }
  pub fn add_employee(&self,e:&Employee,now:&str)->Result<(),DbError>{let c=self.connect()?;c.execute("INSERT INTO employees(id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![e.id,e.employee_number,e.first_name,e.last_name,e.email,e.phone,e.department,e.position,e.hire_date,e.status,now,now])?;Ok(())}
  pub fn list_employees(&self)->Result<Vec<Employee>,DbError>{let c=self.connect()?;let mut s=c.prepare("SELECT id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status FROM employees ORDER BY first_name,last_name")?;let rows=s.query_map([],|r|Ok(Employee{id:r.get(0)?,employee_number:r.get(1)?,first_name:r.get(2)?,last_name:r.get(3)?,email:r.get(4)?,phone:r.get(5)?,department:r.get(6)?,position:r.get(7)?,hire_date:r.get(8)?,status:r.get(9)?}))?;Ok(rows.collect::<Result<Vec<_>,_>>()?)}
  pub fn record_attendance(&self,id:&str,employee_id:&str,date:&str,check_in:&str,token_id:&str,payload:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;tx.execute("INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?,?,?,?,'present',?,?)",params![id,employee_id,date,check_in,token_id,check_in])?;tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','attendance',?,?,?)",params![format!("sync-{id}"),id,payload,check_in])?;tx.commit()?;Ok(())}
}

pub fn database_filename()->&'static str{"natra-erp.sqlite3"}
