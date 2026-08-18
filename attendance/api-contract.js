// Turso-backed attendance API contract.
// The deployable server endpoint should implement this contract using a server-side
// Turso/libSQL connection. Never expose Turso credentials to the browser.
export const ATTENDANCE_API_CONTRACT = {
  method: 'POST',
  path: '/api/attendance/verify',
  body: { token: 'opaque-one-time-token' },
  success: { recorded: true, attendanceId: 'uuid', employee: { id: 'employee-id', displayName: 'Employee' }, checkInAt: 'ISO-8601' },
  errors: ['INVALID_TOKEN', 'EXPIRED_TOKEN', 'TOKEN_ALREADY_USED', 'EMPLOYEE_INACTIVE', 'ALREADY_ATTENDED_TODAY']
};
