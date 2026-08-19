use rusqlite::Connection;

/// Installs database-level employee integrity rules.
/// These rules complement UI validation so direct SQL cannot bypass them.
pub fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(r#"
        UPDATE employees SET
            employee_number = trim(employee_number),
            first_name = trim(first_name),
            last_name = trim(last_name),
            email = CASE WHEN email IS NULL THEN NULL ELSE lower(trim(email)) END,
            phone = CASE WHEN phone IS NULL THEN NULL ELSE trim(phone) END,
            department = CASE WHEN department IS NULL THEN NULL ELSE trim(department) END,
            position = CASE WHEN position IS NULL THEN NULL ELSE trim(position) END,
            hire_date = CASE WHEN hire_date IS NULL THEN NULL ELSE trim(hire_date) END,
            status = lower(trim(status));

        CREATE UNIQUE INDEX IF NOT EXISTS idx_employees_number_normalized
            ON employees(lower(trim(employee_number)));

        CREATE UNIQUE INDEX IF NOT EXISTS idx_employees_email_normalized
            ON employees(lower(trim(email))) WHERE email IS NOT NULL AND trim(email) <> '';

        CREATE TRIGGER IF NOT EXISTS trg_employee_insert_validate
        BEFORE INSERT ON employees
        BEGIN
            SELECT CASE WHEN length(trim(NEW.employee_number)) = 0
                THEN RAISE(ABORT, 'Employee number is required.') END;
            SELECT CASE WHEN NEW.employee_number <> trim(NEW.employee_number)
                THEN RAISE(ABORT, 'Employee number cannot contain leading or trailing spaces.') END;
            SELECT CASE WHEN length(trim(NEW.first_name)) = 0
                THEN RAISE(ABORT, 'First name is required.') END;
            SELECT CASE WHEN length(trim(NEW.last_name)) = 0
                THEN RAISE(ABORT, 'Last name is required.') END;
            SELECT CASE WHEN lower(trim(NEW.status)) NOT IN ('active','inactive')
                THEN RAISE(ABORT, 'Employee status must be active or inactive.') END;
            SELECT CASE WHEN NEW.email IS NOT NULL AND trim(NEW.email) <> '' AND
                (NEW.email <> trim(NEW.email) OR instr(trim(NEW.email), ' ') > 0 OR
                 instr(trim(NEW.email), '@') <= 1 OR instr(substr(trim(NEW.email), instr(trim(NEW.email), '@') + 1), '@') > 0 OR
                 instr(substr(trim(NEW.email), instr(trim(NEW.email), '@') + 1), '.') = 0)
                THEN RAISE(ABORT, 'Invalid email address.') END;
            SELECT CASE WHEN NEW.hire_date IS NOT NULL AND trim(NEW.hire_date) <> '' AND
                (length(trim(NEW.hire_date)) <> 10 OR substr(trim(NEW.hire_date),5,1) <> '-' OR
                 substr(trim(NEW.hire_date),8,1) <> '-' OR date(trim(NEW.hire_date)) IS NULL)
                THEN RAISE(ABORT, 'Hire date must be YYYY-MM-DD.') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_employee_update_validate
        BEFORE UPDATE OF employee_number, first_name, last_name, email, phone, department, position, hire_date, status ON employees
        BEGIN
            SELECT CASE WHEN length(trim(NEW.employee_number)) = 0
                THEN RAISE(ABORT, 'Employee number is required.') END;
            SELECT CASE WHEN NEW.employee_number <> trim(NEW.employee_number)
                THEN RAISE(ABORT, 'Employee number cannot contain leading or trailing spaces.') END;
            SELECT CASE WHEN length(trim(NEW.first_name)) = 0
                THEN RAISE(ABORT, 'First name is required.') END;
            SELECT CASE WHEN length(trim(NEW.last_name)) = 0
                THEN RAISE(ABORT, 'Last name is required.') END;
            SELECT CASE WHEN lower(trim(NEW.status)) NOT IN ('active','inactive')
                THEN RAISE(ABORT, 'Employee status must be active or inactive.') END;
            SELECT CASE WHEN NEW.email IS NOT NULL AND trim(NEW.email) <> '' AND
                (NEW.email <> trim(NEW.email) OR instr(trim(NEW.email), ' ') > 0 OR
                 instr(trim(NEW.email), '@') <= 1 OR instr(substr(trim(NEW.email), instr(trim(NEW.email), '@') + 1), '@') > 0 OR
                 instr(substr(trim(NEW.email), instr(trim(NEW.email), '@') + 1), '.') = 0)
                THEN RAISE(ABORT, 'Invalid email address.') END;
            SELECT CASE WHEN NEW.hire_date IS NOT NULL AND trim(NEW.hire_date) <> '' AND
                (length(trim(NEW.hire_date)) <> 10 OR substr(trim(NEW.hire_date),5,1) <> '-' OR
                 substr(trim(NEW.hire_date),8,1) <> '-' OR date(trim(NEW.hire_date)) IS NULL)
                THEN RAISE(ABORT, 'Hire date must be YYYY-MM-DD.') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_employee_deactivation
        AFTER UPDATE OF status ON employees
        WHEN lower(trim(NEW.status)) = 'inactive' AND lower(trim(OLD.status)) <> 'inactive'
        BEGIN
            UPDATE users SET active=0, updated_at=NEW.updated_at WHERE employee_id=NEW.id;
            INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at)
            SELECT 'sync-user-' || u.id, 'upsert', 'user', u.id,
                   json_object('id',u.id,'username',u.username,'password_hash',u.password_hash,
                               'role',u.role,'employee_id',u.employee_id,'active',0,
                               'must_change_password',u.must_change_password), NEW.updated_at
            FROM users u WHERE u.employee_id=NEW.id
            ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL;
        END;
    "#)?;
    Ok(())
}
