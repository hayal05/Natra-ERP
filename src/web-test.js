const STORAGE_KEY = 'natra.webtest.state.v1';

const seedEmployees = [
  { id: 'EMP-001', name: 'Abebe Kebede', department: 'Operations', status: 'active', phone: '+251 91 000 0001' },
  { id: 'EMP-002', name: 'Sara Tesfaye', department: 'Finance', status: 'active', phone: '+251 91 000 0002' },
  { id: 'EMP-003', name: 'Dawit Alemu', department: 'Sales', status: 'active', phone: '+251 91 000 0003' },
  { id: 'EMP-004', name: 'Mimi Bekele', department: 'HR', status: 'active', phone: '+251 91 000 0004' }
];

const state = loadState();

function loadState() {
  try {
    return { page: 'Dashboard', employees: seedEmployees, attendance: [], ...JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') };
  } catch {
    return { page: 'Dashboard', employees: seedEmployees, attendance: [] };
  }
}
function saveState() { localStorage.setItem(STORAGE_KEY, JSON.stringify(state)); }
function today() { return new Date().toISOString().slice(0, 10); }
function esc(v = '') { return String(v).replace(/[&<>'"]/g, c => ({ '&':'&amp;', '<':'&lt;', '>':'&gt;', "'":'&#39;', '"':'&quot;' }[c])); }
function presentCount() { return state.attendance.filter(a => a.date === today()).length; }
function layout(content) {
  document.querySelector('#app').innerHTML = `<div class="shell web-shell">
    <aside class="sidebar">
      <div class="brand"><div class="brand-mark">N</div><div><strong>NATRA</strong><span>ERP</span></div></div>
      <div class="role-pill"><span></span>HR Admin · WEB TEST</div>
      <nav>${['Dashboard','Employees','Attendance','Leave','Payroll','Documents','Reports','Settings'].map(p => `<button class="nav-item ${state.page===p?'active':''}" data-page="${p}"><i>${icons[p]}</i>${p}</button>`).join('')}</nav>
      <div class="sidebar-bottom"><button class="nav-item" id="reset"><i>↻</i>Reset test data</button></div>
    </aside>
    <main class="main"><header class="topbar"><div><p class="eyebrow">MOBILE WEB TEST</p><h1>${esc(state.page)}</h1></div><div class="profile"><div class="avatar">AD</div><div><strong>Admin</strong><small>HR Admin · Demo</small></div></div></header><div id="page">${content}</div></main>
  </div>`;
  document.querySelectorAll('[data-page]').forEach(b => b.onclick = () => { state.page = b.dataset.page; saveState(); render(); });
  document.querySelector('#reset').onclick = () => { localStorage.removeItem(STORAGE_KEY); location.reload(); };
}
const icons = { Dashboard:'▦', Employees:'◉', Attendance:'◷', Leave:'◫', Payroll:'▤', Documents:'□', Reports:'⌁', Settings:'⚙' };

function login() {
  document.querySelector('#app').innerHTML = `<main class="login-page web-login"><form class="login-card" id="loginForm">
    <div class="brand"><div class="brand-mark">N</div><div><strong>NATRA</strong><span>ERP</span></div></div>
    <span class="eyebrow">MOBILE WEB TEST</span><h1>Sign in</h1><p>Browser-only testing environment.</p>
    <div class="web-note">Demo credentials: <b>admin</b> / <b>Admin@123</b></div>
    <label>Username<input name="username" autocomplete="username" value="admin" required></label>
    <label>Password<input name="password" type="password" autocomplete="current-password" value="Admin@123" required></label>
    <button class="primary green-btn full">Sign in to test</button>
  </form></main>`;
  document.querySelector('#loginForm').onsubmit = e => { e.preventDefault(); const d = Object.fromEntries(new FormData(e.currentTarget)); if (d.username.toLowerCase() === 'admin' && d.password === 'Admin@123') { sessionStorage.setItem('natra.webtest.auth','1'); state.page='Dashboard'; render(); } else { document.querySelector('.web-note').textContent='Invalid demo credentials.'; } };
}

function dashboard() { return `<section class="content"><div class="welcome"><div><span class="badge">WEB TEST</span><h2>Good morning, Admin.</h2><p>Test the NATRA ERP interface from your phone without the Windows/Tauri runtime.</p></div><button class="primary" data-page="Employees">+ Add employee</button></div>
<div class="stats">${stat('Employees',state.employees.length,'Active workforce')}${stat('Present today',presentCount(),'Browser test check-ins')}${stat('On leave','2','Demo records')}${stat('Pending requests','3','Demo records')}</div>
<div class="grid-two"><div class="panel"><div class="panel-head"><div><span class="eyebrow">TODAY</span><h3>Attendance overview</h3></div><button data-page="Attendance">View all →</button></div><div class="progress"><div style="width:${Math.min(100,presentCount()/Math.max(1,state.employees.length)*100)}%"></div></div><div class="legend"><span>Present <b>${presentCount()}</b></span><span>Employees <b>${state.employees.length}</b></span></div></div><div class="panel"><div class="panel-head"><div><span class="eyebrow">MODULES</span><h3>Quick access</h3></div></div>${['Employees','Attendance','Leave','Payroll'].map(p=>`<div class="action"><span class="dot"></span><div><strong>${p}</strong><small>Open ${p.toLowerCase()} module</small></div><button data-page="${p}">Open</button></div>`).join('')}</div></div></section>`; }
function stat(title,value,sub){ return `<div class="stat"><span class="eyebrow">${title}</span><strong>${value}</strong><small>${sub}</small></div>`; }
function employees(){ return `<section class="content"><div class="page-heading"><div><span class="eyebrow">WORKFORCE</span><h2>Employees</h2><p>Browser test records stored locally on this phone.</p></div><button class="primary" id="addEmployee">+ Add employee</button></div><div class="panel table-panel"><div class="table-wrap"><table><thead><tr><th>ID</th><th>Employee</th><th>Department</th><th>Status</th><th>Phone</th></tr></thead><tbody>${state.employees.map(e=>`<tr><td>${esc(e.id)}</td><td><strong>${esc(e.name)}</strong></td><td>${esc(e.department)}</td><td><span class="status present">${esc(e.status)}</span></td><td>${esc(e.phone)}</td></tr>`).join('')}</tbody></table></div></div></section>`; }
function attendance(){ return `<section class="content"><div class="welcome"><div><span class="badge">ATTENDANCE TEST</span><h2>One-time QR attendance</h2><p>Generate a browser demo token and simulate check-in. Real QR/Tauri validation remains unchanged.</p></div><button class="primary" id="checkin">Simulate check-in</button></div><div class="grid-two"><div class="panel qr-panel"><div class="panel-head"><div><span class="eyebrow">DEMO TOKEN</span><h3>Attendance QR</h3></div><span class="status present">Browser only</span></div><div class="qr-placeholder" id="qr">SCAN</div><p class="qr-help">A demo token is generated locally for testing the UI.</p></div><div class="panel"><div class="panel-head"><div><span class="eyebrow">TODAY</span><h3>Check-ins</h3></div></div>${state.attendance.length ? state.attendance.slice().reverse().map(a=>`<div class="action"><span class="dot"></span><div><strong>${esc(a.name)}</strong><small>${esc(a.time)}</small></div><span class="status present">Present</span></div>`).join('') : '<div class="empty">No test check-ins yet.</div>'}</div></div></section>`; }
function simplePage(title, label, text){ return `<section class="content"><div class="page-heading"><div><span class="eyebrow">${label}</span><h2>${title}</h2><p>${text}</p></div></div><div class="grid-two"><div class="panel"><div class="panel-head"><div><span class="eyebrow">WEB TEST</span><h3>${title} module</h3></div><span class="status neutral">Demo</span></div><p>This page is available for responsive navigation testing. Its production data and Tauri commands are intentionally not connected in browser mode.</p></div><div class="panel"><div class="panel-head"><div><span class="eyebrow">NEXT STEP</span><h3>Desktop runtime</h3></div></div><p>Use the Windows build to test the native database, Turso sync, QR validation and secure authentication.</p></div></div></section>`; }
function settings(){ return simplePage('Settings','ADMINISTRATION','Browser test settings. Native secrets and database controls are disabled here for safety.'); }

function render(){
  if (!sessionStorage.getItem('natra.webtest.auth')) return login();
  const pages = { Dashboard:dashboard, Employees:employees, Attendance:attendance, Leave:()=>simplePage('Leave','LEAVE MANAGEMENT','Review the leave-management navigation and responsive layout.'), Payroll:()=>simplePage('Payroll','PAYROLL','Review the payroll navigation and responsive layout.'), Documents:()=>simplePage('Documents','DOCUMENTS','Review the documents navigation and responsive layout.'), Reports:()=>simplePage('Reports','REPORTS','Review the reports navigation and responsive layout.'), Settings:settings };
  layout(pages[state.page] ? pages[state.page]() : dashboard());
  document.querySelectorAll('[data-page]').forEach(b => b.onclick = () => { state.page=b.dataset.page; saveState(); render(); });
  document.querySelector('#addEmployee')?.addEventListener('click', () => { const n = prompt('Employee name'); if(!n) return; state.employees.push({id:`EMP-${String(state.employees.length+1).padStart(3,'0')}`,name:n,department:'General',status:'active',phone:'—'}); saveState(); render(); });
  document.querySelector('#checkin')?.addEventListener('click', () => { const employee=state.employees[0]; const now=new Date(); state.attendance.push({name:employee.name,date:today(),time:now.toLocaleTimeString([], {hour:'2-digit',minute:'2-digit'})}); saveState(); document.querySelector('#qr').textContent='✓'; render(); });
}

const style = document.createElement('style');
style.textContent = `#web-test-banner{position:fixed;z-index:9999;top:0;left:0;right:0;text-align:center;padding:6px 10px;background:#8a2430;color:#fff;font:700 10px/1.2 system-ui;letter-spacing:.06em}.web-shell{padding-top:28px}.web-note{padding:10px 12px;margin:0 0 16px;background:#eef8f4;border:1px solid #cfe9df;border-radius:6px;color:#216b53;font-size:11px}.web-login{padding-top:48px}.web-login .login-card{margin-top:12px}@media(max-width:800px){.web-shell .sidebar{position:fixed;left:-280px;transition:left .2s}.web-shell .main{margin-left:0}.web-shell .topbar{padding:16px}.web-shell .content{padding:16px}.web-shell .grid-two{grid-template-columns:1fr}.web-shell .stats{grid-template-columns:1fr 1fr}.web-shell .table-wrap{overflow-x:auto}.web-shell table{min-width:650px}.web-shell .welcome{flex-direction:column;align-items:flex-start}.web-shell .welcome button{width:100%}.web-login .login-card{width:min(390px,calc(100vw - 28px));padding-left:20px;padding-right:20px}.web-login .login-card:before{margin-left:-20px;margin-right:-20px}}`;
document.head.appendChild(style);

render();
