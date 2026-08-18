use tauri::State;
use crate::db::{Database,Employee};
use argon2::{Argon2,PasswordHash,PasswordVerifier};

#[derive(serde::Deserialize)] pub struct LoginRequest{pub username:String,pub password:String}
#[derive(serde::Serialize)] pub struct LoginResponse{pub id:String,pub username:String,pub role:String,pub employee_id:Option<String>}
#[derive(serde::Deserialize)] pub struct TursoSaveRequest{pub database_url:String,pub auth_token:String}

#[tauri::command] pub fn database_status(db:State<'_,Database>)->Result<String,String>{db.list_employees().map(|_|"sqlite-ready".into()).map_err(|e|e.to_string())}
#[tauri::command] pub fn employees_list(db:State<'_,Database>)->Result<Vec<Employee>,String>{db.list_employees().map_err(|e|e.to_string())}
#[tauri::command] pub fn create_employee(db:State<'_,Database>,employee:Employee)->Result<(),String>{db.add_employee(&employee,&now()).map_err(|e|e.to_string())}
#[tauri::command] pub fn update_employee(db:State<'_,Database>,employee:Employee)->Result<(),String>{db.update_employee(&employee,&now()).map_err(|e|e.to_string())}
#[tauri::command] pub fn record_attendance(db:State<'_,Database>,id:String,employee_id:String,date:String,check_in:String,token_id:String,payload:String)->Result<(),String>{db.record_attendance(&id,&employee_id,&date,&check_in,&token_id,&payload).map_err(|e|e.to_string())}
#[tauri::command] pub fn attendance_today(db:State<'_,Database>,date:String)->Result<Vec<crate::db::AttendanceRow>,String>{db.attendance_today(&date).map_err(|e|e.to_string())}
#[tauri::command] pub fn login(db:State<'_,Database>,request:LoginRequest)->Result<LoginResponse,String>{let (id,stored,employee_id,role)=db.authenticate_user(&request.username).map_err(|_|"Invalid username or password".to_string())?;let parsed=PasswordHash::new(&stored).map_err(|_|"Invalid username or password".to_string())?;Argon2::default().verify_password(request.password.as_bytes(),&parsed).map_err(|_|"Invalid username or password".to_string())?;Ok(LoginResponse{id,username:request.username,role,employee_id})}

#[tauri::command]
pub fn turso_status() -> Result<crate::turso::TursoConfig,String> { crate::turso::status() }
#[tauri::command]
pub fn turso_save(request:TursoSaveRequest) -> Result<(),String> { crate::turso::save(&request.database_url,&request.auth_token) }
#[tauri::command]
pub fn turso_disconnect() -> Result<(),String> { crate::turso::clear() }
#[tauri::command]
pub fn turso_test_connection() -> Result<String,String> { let (url,token)=crate::turso::credentials()?; if url.is_empty()||token.is_empty(){return Err("Turso is not configured".into())} if !url.starts_with("libsql://")&&!url.starts_with("https://"){return Err("Invalid Turso URL".into())} Ok("credentials-ready".into()) }
fn now()->String{use std::time::{SystemTime,UNIX_EPOCH};SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string()}
