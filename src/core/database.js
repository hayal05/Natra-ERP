/** Offline-first database boundary.
 * The UI talks to this adapter, not directly to Turso. A native Windows
 * SQLite implementation can replace this module without changing modules.
 */
import { storage } from './storage.js';

const DB_VERSION = 1;
const META_KEY = 'database-meta';

export const database = {
  version: DB_VERSION,
  migrate() {
    const meta = storage.read(META_KEY, { version: 0 });
    if (meta.version < DB_VERSION) storage.write(META_KEY, { version: DB_VERSION, migratedAt: new Date().toISOString() });
  },
  get(table, fallback = []) { return storage.read(`table:${table}`, fallback); },
  put(table, value) { storage.write(`table:${table}`, value); },
  transaction(work) { return work(this); }
};

database.migrate();
