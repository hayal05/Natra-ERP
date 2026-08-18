/** Lightweight QR transport contract.
 * The payload is deliberately signed/opaque at the application boundary.
 * A QR library can consume encodeAttendancePayload() and a camera scanner can
 * pass the decoded string to decodeAttendancePayload().
 */
export function encodeAttendancePayload(token) {
  if (!token?.id || !token?.employeeId || !token?.expiresAt) throw new Error('Invalid attendance token');
  return JSON.stringify({v:1,t:token.id,e:token.employeeId,x:token.expiresAt,n:token.nonce});
}

export function decodeAttendancePayload(raw) {
  try {
    const p=JSON.parse(raw);
    if(p?.v!==1||!p.t||!p.e||!p.x||!p.n) throw new Error('Invalid QR');
    return {id:p.t,employeeId:p.e,expiresAt:p.x,nonce:p.n};
  } catch { throw new Error('Invalid attendance QR'); }
}
