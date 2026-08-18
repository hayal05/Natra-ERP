mod commands;
mod db;

use commands::DbState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DbState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::database_schema,
            commands::database_status,
            commands::create_employee,
            commands::record_attendance
        ])
        .run(tauri::generate_context!())
        .expect("error while running NATRA ERP");
}
