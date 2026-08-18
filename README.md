# NATRA ERP

A scalable Windows-first company ERP for **HR Admin** and **Employee** users.

## Initial scope

- HR Admin and Employee roles
- Employee management
- Employee self-generated one-time QR attendance
- Automatic attendance date/time recording
- Leave management foundation
- Payroll foundation
- Employee documents
- Reports and audit history
- Offline-first local data with cloud synchronization planned
- Turso-ready data architecture

## Brand

- Product: NATRA ERP
- Primary color: Aviation Green

## Architecture direction

The application is designed for a Windows desktop deployment with a local database for resilient offline operation and a secure synchronization/API layer for centralized cloud data. Turso will be used as the cloud database layer; files/documents will use object storage rather than being stored directly as database blobs.

## Development principles

1. Keep business logic separate from presentation.
2. Make attendance idempotent: one successful check-in per employee per workday.
3. Use short-lived, one-time attendance tokens rather than putting employee data in QR codes.
4. Never expose cloud database credentials to the desktop UI.
5. Preserve local data across application rebuilds and updates.
6. Build modules so future payroll, leave, documents, and reporting features can be added without rewriting the core.
