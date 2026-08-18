mod commands;
mod db;
mod attendance;
mod leave;
mod payroll;
mod sync;
mod turso;
mod autosync;
mod backup;

use tauri::Manager;

#[derive(Debug, serde::Serialize)]
struct PayrollPayslip {
    salary_id: String,
    employee_id: String,
    employee_number: String,
    employee_name: String,
    department: Option<String>,
    position: Option<String>,
    pay_period: String,
    base_salary: f64,
    allowances: f64,
    deductions: f64,
    gross_salary: f64,
    net_salary: f64,
    status: String,
    processed_at: Option<String>,
    processed_by: Option<String>,
}

#[tauri::command]
pub fn payroll_payslip(db: tauri::State<'_, db::Database>, salary_id: String) -> Result<PayrollPayslip, String> {
    let conn = rusqlite::Connection::open(db.path()).map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT s.id,s.employee_id,e.employee_number,e.first_name || ' ' || e.last_name,e.department,e.position,s.pay_period,s.base_salary,s.allowances,s.deductions,s.net_salary,s.status,p.processed_at,p.processed_by FROM salary_records s JOIN employees e ON e.id=s.employee_id LEFT JOIN payroll_periods p ON p.period=s.pay_period WHERE s.id=?",
        [&salary_id],
        |r| Ok(PayrollPayslip {
            salary_id: r.get(0)?,
            employee_id: r.get(1)?,
            employee_number: r.get(2)?,
            employee_name: r.get(3)?,
            department: r.get(4)?,
            position: r.get(5)?,
            pay_period: r.get(6)?,
            base_salary: r.get(7)?,
            allowances: r.get(8)?,
            deductions: r.get(9)?,
            gross_salary: r.get::<_, f64>(7)? + r.get::<_, f64>(8)?,
            net_salary: r.get(10)?,
            status: r.get(11)?,
            processed_at: r.get(12)?,
            processed_by: r.get(13)?,
        }),
    ).map_err(|e| e.to_string())?;
    if result.status != "processed" {
        return Err("Payslip is available only after the payroll period has been processed and locked.".into());
    }
    Ok(result)
}

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
            payroll_payslip,
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
        .run(tauri::generate_context!())
        .expect("error while running NATRA ERP");
}