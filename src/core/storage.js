const PREFIX = 'natra-erp:';

export const storage = {
  read(key, fallback) {
    try { const raw = localStorage.getItem(PREFIX + key); return raw ? JSON.parse(raw) : fallback; }
    catch { return fallback; }
  },
  write(key, value) {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  }
};

export function loadEmployees(seed) {
  return storage.read('employees', seed);
}

export function saveEmployees(employees) {
  storage.write('employees', employees);
}

export function loadAttendance() {
  return storage.read('attendance', []);
}

export function saveAttendance(records) {
  storage.write('attendance', records);
}

export function clearDemoData() {
  Object.keys(localStorage).filter(k => k.startsWith(PREFIX)).forEach(k => localStorage.removeItem(k));
}
