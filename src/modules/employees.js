export function employeeModule(employees, onChange) {
  let editing = null;
  const root = document.createElement('section'); root.className = 'content';
  function draw() {
    root.innerHTML = `<div class="page-heading"><div><span class="eyebrow">WORKFORCE</span><h2>Employees</h2><p>Manage employee records, roles and employment status.</p></div><button class="primary green-btn" id="addEmployee">+ Add employee</button></div><div class="toolbar"><input id="employeeSearch" placeholder="Search by name, ID or department…"><select id="statusFilter"><option value="all">All statuses</option><option value="active">Active</option><option value="inactive">Inactive</option></select></div><div class="panel table-panel"><table><thead><tr><th>Employee</th><th>ID</th><th>Department</th><th>Position</th><th>Status</th><th></th></tr></thead><tbody id="employeeRows"></tbody></table></div><div id="employeeModal"></div>`;
    renderRows(); root.querySelector('#employeeSearch').addEventListener('input', renderRows); root.querySelector('#statusFilter').addEventListener('change', renderRows); root.querySelector('#addEmployee').addEventListener('click', () => openForm());
  }
  function renderRows() {
    const q = root.querySelector('#employeeSearch')?.value.toLowerCase() || ''; const s = root.querySelector('#statusFilter')?.value || 'all';
    const filtered = employees.filter(e => `${e.firstName} ${e.lastName} ${e.employeeNumber} ${e.department || ''}`.toLowerCase().includes(q) && (s === 'all' || e.status === s));
    root.querySelector('#employeeRows').innerHTML = filtered.map(e => `<tr><td><div class="employee-cell"><span class="mini-avatar">${e.firstName[0]}${e.lastName[0]}</span><strong>${e.firstName} ${e.lastName}</strong></div></td><td>${e.employeeNumber}</td><td>${e.department || '—'}</td><td>${e.position || '—'}</td><td><span class="status ${e.status === 'active' ? 'present' : 'inactive'}">${e.status}</span></td><td><button class="row-action" data-edit="${e.id}">Edit</button></td></tr>`).join('') || `<tr><td colspan="6" class="empty">No employees found.</td></tr>`;
    root.querySelectorAll('[data-edit]').forEach(b => b.addEventListener('click', () => openForm(employees.find(e => e.id === b.dataset.edit))));
  }
  function openForm(employee = null) {
    editing = employee; root.querySelector('#employeeModal').innerHTML = `<div class="modal-backdrop"><form class="modal" id="employeeForm"><div class="modal-head"><div><span class="eyebrow">EMPLOYEE RECORD</span><h3>${employee ? 'Edit employee' : 'Add employee'}</h3></div><button type="button" class="close" id="closeModal">×</button></div><div class="form-grid">${field('First name','firstName',employee?.firstName || '')}${field('Last name','lastName',employee?.lastName || '')}${field('Employee ID','employeeNumber',employee?.employeeNumber || '')}${field('Email','email',employee?.email || '','email')}${field('Department','department',employee?.department || '')}${field('Position','position',employee?.position || '')}${field('Phone','phone',employee?.phone || '')}${field('Hire date','hireDate',employee?.hireDate || '','date')}</div><div class="modal-actions"><button type="button" class="secondary" id="cancelModal">Cancel</button><button class="primary green-btn">${employee ? 'Save changes' : 'Create employee'}</button></div></form></div>`;
    root.querySelector('#closeModal').onclick = closeForm; root.querySelector('#cancelModal').onclick = closeForm; root.querySelector('#employeeForm').onsubmit = save;
  }
  function save(ev) { ev.preventDefault(); const data = Object.fromEntries(new FormData(ev.currentTarget)); if (!data.firstName || !data.lastName || !data.employeeNumber) return; if (editing) Object.assign(editing, data, { updatedAt: new Date().toISOString() }); else employees.push({ id: crypto.randomUUID(), ...data, status: 'active', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() }); onChange(employees); closeForm(); draw(); }
  function closeForm() { root.querySelector('#employeeModal').innerHTML = ''; editing = null; }
  draw(); return root;
}
function field(label,name,value,type='text') { const required = ['firstName','lastName','employeeNumber'].includes(name) ? 'required' : ''; return `<label>${label}<input name="${name}" type="${type}" value="${value}" ${required}></label>`; }
