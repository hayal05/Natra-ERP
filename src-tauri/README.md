# NATRA ERP — Windows data layer

This directory is reserved for the native Windows/Tauri persistence adapter.

Target architecture:

- UI: Vite/JavaScript
- Local database: SQLite stored in the app data directory
- Sync: durable outbox queue
- Cloud: Turso/libSQL adapter
- Authentication: role-based HR Admin / Employee

The browser storage adapter remains the development fallback. The native adapter
must never store the SQLite database inside the repository or replace it during
application rebuilds.

Migration policy:
1. Database lives in the OS application-data directory.
2. Schema migrations are versioned and additive where possible.
3. Every cloud mutation is written to the local outbox first.
4. Sync retries safely and is idempotent using operation IDs.
5. Failed sync never blocks local attendance or employee operations.
