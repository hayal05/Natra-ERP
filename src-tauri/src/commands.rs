use tauri::State;
use crate::db::{Database, Employee};

#[tauri::command]
pub fn database_schema() -> String { crate::db::SCHEMA.to_string() }

#[tauri::command]
pub fn database_status(db: State<'_, Database>) -> Result<String, String> { db.list_employees().map(|_| "sqlite-ready".to_string()).map_err(|e| e.to_string()) }

#[tauri::command]
pub fn employees_list(db: State<'_, Database>) -> Result<Vec<Employee>, String> { db.list_employees().map_err(|e| e.to_string()) }

#[tauri::command]
pub fn create_employee(db: State<'_, Database>, employee: Employee) -> Result<(), String> { db.add_employee(&employee, &now()).map_err(|e| e.to_string()) }

#[tauri::command]
pub fn update_employee(db: State<'_, Database>, employee: Employee) -> Result<(), String> { db.update_employee(&employee, &now()).map_err(|e| e.to_string()) }

fn now() -> String { use std::time::{SystemTime, UNIX_EPOCH}; SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string() }
