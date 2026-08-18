const state = document.querySelector('#state');
const token = decodeURIComponent(location.pathname.split('/').filter(Boolean).pop() || '');
const API = '/api/attendance/verify';

const messages = {
  INVALID_TOKEN:'This attendance QR is invalid.', EXPIRED_TOKEN:'This attendance QR has expired. Please generate a new one.',
  TOKEN_ALREADY_USED:'This attendance QR has already been used.', EMPLOYEE_INACTIVE:'This employee is not active.',
  ALREADY_ATTENDED_TODAY:'Attendance has already been recorded today.'
};

async function verify() {
  if (!token) throw new Error('INVALID_TOKEN');
  const response = await fetch(API,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({token})});
  const data = await response.json().catch(()=>({}));
  if (!response.ok) throw new Error(data.code || 'VERIFICATION_FAILED');
  return data;
}

(async()=>{
  state.textContent='Verifying attendance securely…';
  try {
    const result=await verify();
    state.className='state ok';
    state.innerHTML=`<strong>✓ Attendance recorded</strong><br>${escapeHtml(result.employee?.displayName||'Employee')}<br>${new Date(result.checkInAt).toLocaleString()}`;
  } catch(error) {
    state.className='state bad';
    state.textContent=messages[error.message]||'Attendance could not be verified. Please try again.';
  }
})();
function escapeHtml(value){return String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
