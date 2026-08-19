mod attendance;
mod autosync;
mod backup;
mod commands;
mod db;
mod employee_hardening;
mod leave;
mod leave_hardening;
mod payroll;
mod security;
mod sync;
mod turso;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("NATRA ERP app data directory unavailable: {e}"))?;
            let database = db::Database::open(&app_data)
                .map_err(|e| format!("NATRA ERP SQLite initialization failed: {e}"))?;
            {
                let conn = rusqlite::Connection::open(database.path())
                    .map_err(|e| format!("NATRA ERP employee database unavailable: {e}"))?;
                employee_hardening::migrate(&conn)
                    .map_err(|e| format!("NATRA ERP employee hardening migration failed: {e}"))?;
                leave::migrate(&conn)
                    .map_err(|e| format!("NATRA ERP leave migration failed: {e}"))?;
                leave_hardening::migrate(&conn)
                    .map_err(|e| format!("NATRA ERP leave hardening migration failed: {e}"))?;
                payroll::migrate(&conn)
                    .map_err(|e| format!("NATRA ERP payroll migration failed: {e}"))?;
            }
            autosync::start(database.path().clone());
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
            commands::attendance_history,
            commands::attendance_summary,
            commands::attendance_delete,
            commands::login,
            commands::change_password,
            commands::logout,
            commands::users_list,
            commands::user_create,
            commands::user_update,
            commands::user_reset_password,
            commands::leave_create,
            commands::leave_list,
            commands::leave_review,
            commands::leave_types_list,
            commands::leave_type_save,
            commands::leave_balances_list,
            commands::leave_balance_save,
            commands::payroll_create,
            commands::payroll_list,
            commands::payroll_period,
            commands::payroll_periods,
            commands::payroll_update,
            commands::payroll_process,
            commands::payroll_payslip,
            commands::turso_status,
            commands::turso_save,
            commands::turso_disconnect,
            commands::turso_test_connection,
            commands::sync_status,
            commands::sync_now,
            commands::backup_create,
            commands::backup_list,
            commands::database_integrity,
            commands::backup_restore
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("NATRA ERP terminated: {error}");
    }
}
