mod commands;
mod db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::database_schema,
            commands::database_status,
            commands::employees_list,
            commands::create_employee,
            commands::update_employee,
            commands::record_attendance
        ])
        .run(tauri::generate_context!())
        .expect("error while running NATRA ERP");
}
