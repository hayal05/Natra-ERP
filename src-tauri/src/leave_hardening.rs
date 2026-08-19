use rusqlite::Connection;

/// Additional database-level safeguards for Leave Management.
/// The existing leave.rs remains responsible for business workflows; these
/// safeguards protect the same invariants if another code path writes locally.
pub fn migrate(c: &Connection) -> Result<(), rusqlite::Error> {
    // Prevent negative/over-used balances even when a caller bypasses the UI.
    c.execute_batch(
        "
        CREATE TRIGGER IF NOT EXISTS trg_leave_balance_insert_valid
        BEFORE INSERT ON leave_balances
        WHEN NEW.allocated_days < 0 OR NEW.used_days < 0 OR NEW.used_days > NEW.allocated_days
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave balance values');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_leave_balance_update_valid
        BEFORE UPDATE OF allocated_days, used_days ON leave_balances
        WHEN NEW.allocated_days < 0 OR NEW.used_days < 0 OR NEW.used_days > NEW.allocated_days
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave balance values');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_leave_type_insert_valid
        BEFORE INSERT ON leave_types
        WHEN trim(NEW.name) = '' OR NEW.annual_days < 0
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave type');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_leave_type_update_valid
        BEFORE UPDATE OF name, annual_days ON leave_types
        WHEN trim(NEW.name) = '' OR NEW.annual_days < 0
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave type');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_leave_request_insert_dates
        BEFORE INSERT ON leave_requests
        WHEN length(NEW.start_date) <> 10
          OR length(NEW.end_date) <> 10
          OR substr(NEW.start_date,5,1) <> '-'
          OR substr(NEW.start_date,8,1) <> '-'
          OR substr(NEW.end_date,5,1) <> '-'
          OR substr(NEW.end_date,8,1) <> '-'
          OR NEW.start_date > NEW.end_date
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave date range');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_leave_request_update_dates
        BEFORE UPDATE OF start_date, end_date ON leave_requests
        WHEN length(NEW.start_date) <> 10
          OR length(NEW.end_date) <> 10
          OR substr(NEW.start_date,5,1) <> '-'
          OR substr(NEW.start_date,8,1) <> '-'
          OR substr(NEW.end_date,5,1) <> '-'
          OR substr(NEW.end_date,8,1) <> '-'
          OR NEW.start_date > NEW.end_date
        BEGIN
            SELECT RAISE(ABORT, 'Invalid leave date range');
        END;
        
        CREATE INDEX IF NOT EXISTS idx_leave_active_employee_dates
          ON leave_requests(employee_id, status, start_date, end_date);
        CREATE INDEX IF NOT EXISTS idx_leave_balance_lookup
          ON leave_balances(employee_id, leave_type_id, year);
        CREATE INDEX IF NOT EXISTS idx_leave_type_name_ci
          ON leave_types(lower(name));
        ",
    )?;
    Ok(())
}
