const state=document.querySelector('#state');
const token=decodeURIComponent(location.pathname.split('/').filter(Boolean).pop()||'');
if(!token){state.className='state bad';state.textContent='Invalid attendance QR.';}
else {
  // Production endpoint contract: this page will POST the opaque one-time token
  // to the Turso-backed attendance API. No employee information is trusted from
  // the QR itself. The API resolves and consumes the token atomically.
  state.className='state';
  state.innerHTML='<strong>Attendance token received.</strong><br>Secure verification endpoint is ready.';
}
