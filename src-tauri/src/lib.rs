mod commands;
mod db;
mod leave;
mod payroll;
mod sync;
mod turso;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("NATRA ERP app data directory unavailable");
            let database = db::Database::open(&app_data).expect("NATRA ERP SQLite initialization failed");
            {
                let conn = rusqlite::Connection::open(database.path()).expect("NATRA ERP leave database unavailable");
                leave::migrate(&conn).expect("NATRA ERP leave migration failed");
                payroll::migrate(&conn).expect("NATRA ERP payroll migration failed");
            }
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::database_status,
            commands::employees_list,
            commands::create_employee,
            commands::update_employee,
            commands::record_attendance,
            commands::attendance_today,
            commands::login,
            commands::leave_create,
            commands::leave_list,
            commands::leave_review,
            commands::payroll_create,
            commands::payroll_list,
            commands::turso_status,
            commands::turso_save,
            commands::turso_disconnect,
            commands::turso_test_connection,
            commands::sync_status,
            commands::sync_now
        ])
        .run(tauri::generate_context!())
        .expect("error while running NATRA ERP");
}
