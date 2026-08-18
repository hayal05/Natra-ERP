use std::sync::Mutex;
use tauri::State;

use crate::db::SCHEMA;

pub struct DbState(pub Mutex<Option<()>>);

#[tauri::command]
pub fn database_schema() -> String {
    SCHEMA.to_string()
}

#[tauri::command]
pub fn database_status() -> String {
    "native-sqlite-ready".to_string()
}

#[tauri::command]
pub fn create_employee(_state: State<DbState>, employee_json: String) -> Result<String, String> {
    if employee_json.trim().is_empty() { return Err("Employee payload is empty".into()); }
    Ok(employee_json)
}

#[tauri::command]
pub fn record_attendance(_state: State<DbState>, attendance_json: String) -> Result<String, String> {
    if attendance_json.trim().is_empty() { return Err("Attendance payload is empty".into()); }
    Ok(attendance_json)
}
