import './styles.css';

const state = {
  role: 'HR Admin',
  active: 'Dashboard'
};

const nav = [
  ['Dashboard', '▦'],
  ['Employees', '◉'],
  ['Attendance', '◷'],
  ['Leave', '◫'],
  ['Payroll', '▤'],
  ['Documents', '□'],
  ['Reports', '⌁']
];

function render() {
  document.querySelector('#app').innerHTML = `
    <div class="shell">
      <aside class="sidebar">
        <div class="brand"><div class="brand-mark">N</div><div><strong>NATRA</strong><span>ERP</span></div></div>
        <div class="role-pill"><span></span>${state.role}</div>
        <nav>${nav.map(([label, icon]) => `<button class="nav-item ${state.active === label ? 'active' : ''}" data-page="${label}"><i>${icon}</i>${label}</button>`).join('')}</nav>
        <div class="sidebar-bottom"><button class="nav-item"><i>⚙</i>Settings</button></div>
      </aside>
      <main class="main">
        <header class="topbar">
          <div><p class="eyebrow">COMPANY OPERATIONS</p><h1>${state.active}</h1></div>
          <div class="profile"><div class="avatar">HA</div><div><strong>HR Admin</strong><small>Administrator</small></div><span>⌄</span></div>
        </header>
        ${dashboard()}
      </main>
    </div>`;

  document.querySelectorAll('[data-page]').forEach(btn => btn.addEventListener('click', () => {
    state.active = btn.dataset.page;
    render();
  }));
}

function dashboard() {
  if (state.active === 'Attendance') return attendance();
  return `<section class="content">
    <div class="welcome"><div><span class="badge">LIVE WORKFORCE</span><h2>Good morning, HR Admin.</h2><p>Manage your workforce from one secure workspace.</p></div><button class="primary" onclick="window.setPage && window.setPage('Employees')">+ Add employee</button></div>
    <div class="stats">
      ${card('Employees','248','Active workforce','+12 this month','up')}
      ${card('Present today','221','Attendance','89.1%','up')}
      ${card('On leave','14','Currently away','5.6%','neutral')}
      ${card('Pending requests','8','Needs review','3 urgent','warn')}
    </div>
    <div class="grid-two"><div class="panel"><div class="panel-head"><div><span class="eyebrow">TODAY</span><h3>Attendance overview</h3></div><button>View all →</button></div><div class="progress"><div style="width:89.1%"></div></div><div class="legend"><span>Present <b>221</b></span><span>Late <b>13</b></span><span>Absent <b>0</b></span></div></div>
    <div class="panel"><div class="panel-head"><div><span class="eyebrow">ACTION CENTER</span><h3>Needs your attention</h3></div></div><div class="action"><span class="dot warn-dot"></span><div><strong>8 leave requests</strong><small>Waiting for approval</small></div><button>Review</button></div><div class="action"><span class="dot"></span><div><strong>3 new employees</strong><small>Profiles incomplete</small></div><button>Open</button></div></div></div>
  </section>`;
}

function attendance() { return `<section class="content"><div class="welcome"><div><span class="badge">ATTENDANCE</span><h2>Today's check-ins</h2><p>One-time QR attendance records the employee and exact check-in time.</p></div><button class="primary">Generate QR session</button></div><div class="panel"><div class="panel-head"><h3>Live attendance</h3><button>Refresh</button></div><table><thead><tr><th>Employee</th><th>Department</th><th>Check-in</th><th>Status</th></tr></thead><tbody><tr><td><strong>Abebe Kebede</strong></td><td>Finance</td><td>08:02:14</td><td><span class="status present">Present</span></td></tr><tr><td><strong>Sarah Tadesse</strong></td><td>Operations</td><td>08:17:42</td><td><span class="status late">Late</span></td></tr><tr><td><strong>Dawit Alemu</strong></td><td>HR</td><td>08:04:08</td><td><span class="status present">Present</span></td></tr></tbody></table></div></section>`; }
function card(a,b,c,d,type){return `<div class="stat"><span class="eyebrow">${a}</span><strong>${b}</strong><div><small>${c}</small><em class="${type}">${d}</em></div></div>`}

window.setPage = page => { state.active = page; render(); };
render();
