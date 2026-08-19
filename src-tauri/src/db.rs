use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
use thiserror::Error;

pub const DB_SCHEMA_VERSION: i32 = 12;
const DEFAULT_ADMIN_PASSWORD_HASH: &str = "$argon2id$v=19$m=65536,t=3,p=4$9+8DePK/MlZJ/0iA2XHylg$jVFn51IEt/eYTkue7hkmbJJlfg1mxsksIV3NwWFxilE";

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("authentication failed")]
    AuthFailed,
    #[error("invalid employee account link: {0}")]
    InvalidEmployeeLink(String),
}

#[derive(Clone)]
pub struct Database { path: PathBuf }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Employee {
    pub id: String,
    pub employee_number: String,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AttendanceRow {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department: Option<String>,
    pub attendance_date: String,
    pub check_in_at: String,
    pub token_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: String,
    pub employee_id: Option<String>,
    pub active: bool,
    pub must_change_password: bool,
}

#[derive(Debug, Serialize)]
struct UserSync {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub employee_id: Option<String>,
    pub active: bool,
    pub must_change_password: bool,
}

impl Database {
    pub fn open(app_data_dir: &Path) -> Result<Self, DbError> {
        fs::create_dir_all(app_data_dir)?;
        let db = Self { path: app_data_dir.join("natra-erp.sqlite3") };
        db.migrate()?;
        Ok(db)
    }

    pub fn path(&self) -> PathBuf { self.path.clone() }

    fn connect(&self) -> Result<Connection, DbError> {
        let c = Connection::open(&self.path)?;
        c.pragma_update(None, "foreign_keys", "ON")?;
        c.pragma_update(None, "journal_mode", "WAL")?;
        c.pragma_update(None, "busy_timeout", 5000)?;
        Ok(c)
    }

    fn migrate(&self) -> Result<(), DbError> {
        let mut c = self.connect()?;
        let tx = c.transaction()?;
        tx.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS schema_meta(version INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS employees(id TEXT PRIMARY KEY,employee_number TEXT NOT NULL UNIQUE,first_name TEXT NOT NULL,last_name TEXT NOT NULL,email TEXT UNIQUE,phone TEXT,department TEXT,position TEXT,hire_date TEXT,status TEXT NOT NULL DEFAULT 'active',created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY,username TEXT NOT NULL UNIQUE,password_hash TEXT NOT NULL,role TEXT NOT NULL CHECK(role IN ('hr_admin','employee')),employee_id TEXT UNIQUE REFERENCES employees(id),active INTEGER NOT NULL DEFAULT 1,must_change_password INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS attendance(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL REFERENCES employees(id),attendance_date TEXT NOT NULL,check_in_at TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'present',token_id TEXT NOT NULL UNIQUE,created_at TEXT NOT NULL,UNIQUE(employee_id,attendance_date));
            CREATE TABLE IF NOT EXISTS sync_outbox(id TEXT PRIMARY KEY,operation TEXT NOT NULL,entity TEXT NOT NULL,entity_id TEXT NOT NULL,payload TEXT NOT NULL,created_at TEXT NOT NULL,attempts INTEGER NOT NULL DEFAULT 0,last_error TEXT);
            CREATE TABLE IF NOT EXISTS sync_conflicts(id TEXT PRIMARY KEY,entity TEXT NOT NULL,entity_id TEXT NOT NULL,local_payload TEXT NOT NULL,remote_updated_at TEXT,detected_at TEXT NOT NULL,resolved INTEGER NOT NULL DEFAULT 0);
            CREATE INDEX IF NOT EXISTS idx_attendance_date ON attendance(attendance_date);
            CREATE INDEX IF NOT EXISTS idx_sync_entity ON sync_outbox(entity,entity_id);
            CREATE INDEX IF NOT EXISTS idx_sync_conflicts_open ON sync_conflicts(resolved,detected_at);
        "#)?;

        let has_column: bool = tx.prepare("SELECT 1 FROM pragma_table_info('users') WHERE name='must_change_password'")
            .ok().and_then(|mut s| s.query_row([], |_| Ok(1)).optional().ok()).flatten().is_some();
        if !has_column { tx.execute("ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0", [])?; }

        let v: Option<i32> = tx.query_row("SELECT version FROM schema_meta LIMIT 1", [], |r| r.get(0)).optional()?;
        let old_version = v.unwrap_or(0);
        match v {
            None => { tx.execute("INSERT INTO schema_meta(version) VALUES (?)", [DB_SCHEMA_VERSION])?; }
            Some(old) if old < DB_SCHEMA_VERSION => { tx.execute("UPDATE schema_meta SET version=?", [DB_SCHEMA_VERSION])?; }
            _ => {}
        }

        let now = now_string();
        tx.execute("INSERT OR IGNORE INTO users(id,username,password_hash,role,employee_id,active,must_change_password,created_at,updated_at) VALUES('admin','admin',?, 'hr_admin',NULL,1,1,?,?)", params![DEFAULT_ADMIN_PASSWORD_HASH, now, now])?;
        if old_version < DB_SCHEMA_VERSION {
            tx.execute("UPDATE users SET username='admin', password_hash=?, must_change_password=1, active=1, role='hr_admin', updated_at=? WHERE lower(username)='admin' AND must_change_password=1", params![DEFAULT_ADMIN_PASSWORD_HASH, now])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn prepare_bootstrap_login(&self, username: &str, password: &str) -> Result<(), DbError> {
        if !username.trim().eq_ignore_ascii_case("admin") || password != "Admin@123" { return Ok(()); }
        let mut c = self.connect()?;
        let tx = c.transaction()?;
        let now = now_string();
        let row: Option<(String, bool, bool, String)> = tx.query_row(
            "SELECT id,must_change_password,active,role FROM users WHERE lower(username)='admin' LIMIT 1",
            [], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get::<_, i64>(2)? != 0, r.get(3)?))
        ).optional()?;
        match row {
            None => { tx.execute("INSERT INTO users(id,username,password_hash,role,employee_id,active,must_change_password,created_at,updated_at) VALUES('admin','admin',?,'hr_admin',NULL,1,1,?,?)", params![DEFAULT_ADMIN_PASSWORD_HASH, now, now])?; }
            Some((_, true, _, _)) => { tx.execute("UPDATE users SET username='admin',password_hash=?,role='hr_admin',active=1,must_change_password=1,updated_at=? WHERE lower(username)='admin' AND must_change_password=1", params![DEFAULT_ADMIN_PASSWORD_HASH, now])?; }
            Some(_) => {}
        }
        tx.commit()?;
        Ok(())
    }

    fn queue(&self, tx: &rusqlite::Transaction<'_>, entity: &str, entity_id: &str, payload: &str, now: &str) -> Result<(), DbError> {
        tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert',?,?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL,attempts=0", params![format!("sync-{entity}-{entity_id}"),entity,entity_id,payload,now])?;
        Ok(())
    }

    fn validate_employee_link(tx: &rusqlite::Transaction<'_>, role: &str, employee_id: Option<&str>) -> Result<(), DbError> {
        if role == "employee" && employee_id.is_none() { return Err(DbError::InvalidEmployeeLink("Employee accounts must be linked to an employee record.".into())); }
        if let Some(id) = employee_id {
            let status: Option<String> = tx.query_row("SELECT status FROM employees WHERE id=?", [id], |r| r.get(0)).optional()?;
            match status {
                Some(s) if s.eq_ignore_ascii_case("active") => Ok(()),
                Some(_) => Err(DbError::InvalidEmployeeLink("The selected employee is inactive. Reactivate the employee before linking a login account.".into())),
                None => Err(DbError::InvalidEmployeeLink("The selected employee does not exist.".into())),
            }
        } else { Ok(()) }
    }

    pub fn add_employee(&self,e:&Employee,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;tx.execute("INSERT INTO employees(id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![e.id,e.employee_number,e.first_name,e.last_name,e.email,e.phone,e.department,e.position,e.hire_date,e.status,now,now])?;let payload=serde_json::to_string(e).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"employee",&e.id,&payload,now)?;tx.commit()?;Ok(())}
    pub fn update_employee(&self,e:&Employee,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;let n=tx.execute("UPDATE employees SET employee_number=?,first_name=?,last_name=?,email=?,phone=?,department=?,position=?,hire_date=?,status=?,updated_at=? WHERE id=?",params![e.employee_number,e.first_name,e.last_name,e.email,e.phone,e.department,e.position,e.hire_date,e.status,now,e.id])?;if n==0{return Err(rusqlite::Error::QueryReturnedNoRows.into())}let payload=serde_json::to_string(e).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"employee",&e.id,&payload,now)?;tx.commit()?;Ok(())}
    pub fn list_employees(&self)->Result<Vec<Employee>,DbError>{let c=self.connect()?;let mut s=c.prepare("SELECT id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status FROM employees ORDER BY first_name,last_name")?;let rows=s.query_map([],|r|Ok(Employee{id:r.get(0)?,employee_number:r.get(1)?,first_name:r.get(2)?,last_name:r.get(3)?,email:r.get(4)?,phone:r.get(5)?,department:r.get(6)?,position:r.get(7)?,hire_date:r.get(8)?,status:r.get(9)?}))?;Ok(rows.collect::<Result<Vec<_>,_>>()?)}
    pub fn authenticate_user(&self,username:&str)->Result<(String,String,Option<String>,String,bool),DbError>{let c=self.connect()?;c.query_row("SELECT u.id,u.password_hash,u.employee_id,u.role,u.must_change_password FROM users u LEFT JOIN employees e ON e.id=u.employee_id WHERE lower(u.username)=lower(?) AND u.active=1 AND (u.employee_id IS NULL OR e.status='active')",[username],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get::<_,i64>(4)? != 0))).optional()?.ok_or(DbError::AuthFailed)}
    pub fn change_password(&self,username:&str,new_hash:&str,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;let n=tx.execute("UPDATE users SET password_hash=?,must_change_password=0,updated_at=? WHERE lower(username)=lower(?) AND active=1",params![new_hash,now,username])?;if n==0{return Err(DbError::AuthFailed)}let row:(String,String,Option<String>,bool)=tx.query_row("SELECT id,role,employee_id,active FROM users WHERE lower(username)=lower(?)",[username],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get::<_,i64>(3)? != 0)))?;let payload=serde_json::to_string(&UserSync{id:row.0.clone(),username:username.into(),password_hash:new_hash.into(),role:row.1,employee_id:row.2,active:row.3,must_change_password:false}).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"user",&row.0,&payload,now)?;tx.commit()?;Ok(())}
    pub fn create_user(&self,id:&str,username:&str,hash:&str,role:&str,employee_id:Option<&str>,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;Self::validate_employee_link(&tx,role,employee_id)?;tx.execute("INSERT INTO users(id,username,password_hash,role,employee_id,must_change_password,created_at,updated_at) VALUES(?,?,?,?,?,1,?,?)",params![id,username,hash,role,employee_id,now,now])?;let payload=serde_json::to_string(&UserSync{id:id.into(),username:username.into(),password_hash:hash.into(),role:role.into(),employee_id:employee_id.map(str::to_owned),active:true,must_change_password:true}).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"user",id,&payload,now)?;tx.commit()?;Ok(())}
    pub fn reset_password(&self,id:&str,new_hash:&str,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;let row:Option<(String,String,Option<String>,bool)>=tx.query_row("SELECT username,role,employee_id,active FROM users WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get::<_,i64>(3)? != 0))).optional()?;let (username,role,employee_id,active)=row.ok_or(DbError::AuthFailed)?;if !active{return Err(DbError::AuthFailed)}if let Some(employee_id)=employee_id.as_deref(){Self::validate_employee_link(&tx,&role,Some(employee_id))?;}tx.execute("UPDATE users SET password_hash=?,must_change_password=1,updated_at=? WHERE id=? AND active=1",params![new_hash,now,id])?;let payload=serde_json::to_string(&UserSync{id:id.into(),username,password_hash:new_hash.into(),role,employee_id,active,must_change_password:true}).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"user",id,&payload,now)?;tx.commit()?;Ok(())}
    pub fn update_user(&self,id:&str,username:&str,role:&str,employee_id:Option<&str>,active:bool,now:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;Self::validate_employee_link(&tx,role,employee_id)?;let n=tx.execute("UPDATE users SET username=?,role=?,employee_id=?,active=?,updated_at=? WHERE id=?",params![username,role,employee_id,active,now,id])?;if n==0{return Err(rusqlite::Error::QueryReturnedNoRows.into())}let row:String=tx.query_row("SELECT password_hash FROM users WHERE id=?",[id],|r|r.get(0))?;let must_change:bool=tx.query_row("SELECT must_change_password FROM users WHERE id=?",[id],|r|Ok(r.get::<_,i64>(0)? != 0))?;let payload=serde_json::to_string(&UserSync{id:id.into(),username:username.into(),password_hash:row,role:role.into(),employee_id:employee_id.map(str::to_owned),active,must_change_password:must_change}).map_err(|_|rusqlite::Error::InvalidQuery)?;self.queue(&tx,"user",id,&payload,now)?;tx.commit()?;Ok(())}
    pub fn list_users(&self)->Result<Vec<User>,DbError>{let c=self.connect()?;let mut s=c.prepare("SELECT id,username,role,employee_id,active,must_change_password FROM users ORDER BY username")?;let rows=s.query_map([],|r|Ok(User{id:r.get(0)?,username:r.get(1)?,role:r.get(2)?,employee_id:r.get(3)?,active:r.get::<_,i64>(4)? != 0,must_change_password:r.get::<_,i64>(5)? != 0}))?;Ok(rows.collect::<Result<Vec<_>,_>>()?)}
    pub fn record_attendance(&self,id:&str,employee_id:&str,date:&str,check_in:&str,token_id:&str,payload:&str)->Result<(),DbError>{let mut c=self.connect()?;let tx=c.transaction()?;tx.execute("INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?,?,?,?,'present',?,?)",params![id,employee_id,date,check_in,token_id,check_in])?;self.queue(&tx,"attendance",id,payload,check_in)?;tx.commit()?;Ok(())}
    pub fn attendance_today(&self,date:&str)->Result<Vec<AttendanceRow>,DbError>{let c=self.connect()?;let mut s=c.prepare("SELECT a.id,a.employee_id,e.first_name||' '||e.last_name,e.department,a.attendance_date,a.check_in_at,a.token_id,a.status FROM attendance a JOIN employees e ON e.id=a.employee_id WHERE a.attendance_date=? ORDER BY a.check_in_at DESC")?;let rows=s.query_map([date],|r|Ok(AttendanceRow{id:r.get(0)?,employee_id:r.get(1)?,employee_name:r.get(2)?,department:r.get(3)?,attendance_date:r.get(4)?,check_in_at:r.get(5)?,token_id:r.get(6)?,status:r.get(7)?}))?;Ok(rows.collect::<Result<Vec<_>,_>>()?)}
}

fn now_string() -> String { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string() }
