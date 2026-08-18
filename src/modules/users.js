const invoke = globalThis.__TAURI__?.core?.invoke;

export async function mountUserManagement(root, currentUser) {
  if (!invoke) {
    root.innerHTML = '<section class="content"><div class="panel"><h3>User management requires the Windows desktop application.</h3><p>The browser/mobile test intentionally does not expose native authentication or database commands.</p></div></section>';
    return;
  }
  let users = [];
  let employees = [];
  let editing = null;
  const esc = value => String(value ?? '').replace(/[&<>\"]/g, ch => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[ch]));
  const draw = () => {
    root.innerHTML = `<section class="content" style="--brand:#138a63">
      <div class="page-heading"><div><span class="eyebrow">ADMINISTRATION</span><h2>User management</h2><p>Create login accounts, assign roles and link employees. New accounts must change their temporary password.</p></div><button class="primary green-btn" id="newUser">+ Add user</button></div>
      <div class="toolbar"><input id="userSearch" placeholder="Search username, role or employee…"><select id="userFilter"><option value="all">All users</option><option value="active">Active</option><option value="inactive">Inactive</option><option value="change">Password change required</option></select></div>
      <div class="panel table-panel"><table><thead><tr><th>User</th><th>Role</th><th>Employee</th><th>Status</th><th>Password</th><th></th></tr></thead><tbody id="userRows"></tbody></table></div>
      <div id="userModal"></div><div id="userMessage" class="token-note"></div>
    </section>`;
    root.querySelector('#newUser').onclick = () => openForm();
    root.querySelector('#userSearch').oninput = renderRows;
    root.querySelector('#userFilter').onchange = renderRows;
    renderRows();
  };
  const renderRows = () => {
    const q = root.querySelector('#userSearch')?.value.toLowerCase() || '';
    const filter = root.querySelector('#userFilter')?.value || 'all';
    const filtered = users.filter(u => {
      const employee = employees.find(e => e.id === u.employee_id);
      const hay = `${u.username} ${u.role} ${employee ? `${employee.first_name || employee.firstName} ${employee.last_name || employee.lastName}` : ''}`.toLowerCase();
      return hay.includes(q) && (filter === 'all' || (filter === 'active' && u.active) || (filter === 'inactive' && !u.active) || (filter === 'change' && u.must_change_password));
    });
    root.querySelector('#userRows').innerHTML = filtered.map(u => {
      const employee = employees.find(e => e.id === u.employee_id);
      const employeeName = employee ? `${employee.first_name || employee.firstName} ${employee.last_name || employee.lastName}` : '—';
      const self = currentUser?.id === u.id;
      return `<tr><td><strong>${esc(u.username)}</strong>${self?'<small style="display:block;color:#718079">Current account</small>':''}</td><td><span class="status ${u.role==='hr_admin'?'present':'neutral'}">${u.role==='hr_admin'?'HR Admin':'Employee'}</span></td><td>${esc(employeeName)}</td><td><span class="status ${u.active?'present':'inactive'}">${u.active?'Active':'Inactive'}</span></td><td>${u.must_change_password?'<span class="status warn">Required</span>':'<span class="status present">Set</span>'}</td><td><button class="row-action" data-edit="${esc(u.id)}">Edit</button></td></tr>`;
    }).join('') || '<tr><td colspan="6" class="empty">No users found.</td></tr>';
    root.querySelectorAll('[data-edit]').forEach(btn => btn.onclick = () => openForm(users.find(u => u.id === btn.dataset.edit)));
  };
  const openForm = user => {
    editing = user || null;
    const employeeOptions = ['<option value="">No employee link</option>', ...employees.map(e => { const id=e.id; const first=e.first_name||e.firstName||''; const last=e.last_name||e.lastName||''; return `<option value="${esc(id)}" ${user?.employee_id===id?'selected':''}>${esc(`${e.employee_number||e.employeeNumber||''} — ${first} ${last}`)}</option>`; })].join('');
    root.querySelector('#userModal').innerHTML = `<div style="position:fixed;inset:0;z-index:9999;background:rgba(8,15,12,.62);display:flex;align-items:center;justify-content:center;padding:20px"><form id="userForm" style="width:min(620px,100%);background:#fff;border-radius:16px;padding:24px;box-shadow:0 24px 80px rgba(0,0,0,.25)"><div style="display:flex;justify-content:space-between;align-items:center"><div><span class="eyebrow">LOGIN ACCOUNT</span><h3>${user?'Edit user':'Create user'}</h3></div><button type="button" id="closeUser" style="border:0;background:transparent;font-size:26px">×</button></div><div class="form-grid"><label>Username<input name="username" value="${esc(user?.username||'')}" required autocomplete="off"></label><label>Role<select name="role"><option value="employee" ${user?.role==='employee'?'selected':''}>Employee</option><option value="hr_admin" ${user?.role==='hr_admin'?'selected':''}>HR Admin</option></select></label><label>Employee link<select name="employee_id">${employeeOptions}</select></label>${user?'':'<label>Temporary password<input name="password" type="password" minlength="8" required autocomplete="new-password"><small>At least 8 characters and one special character. The user will be forced to change it.</small></label>'}<label class="toggle-row"><input name="active" type="checkbox" ${user?.active!==false?'checked':''} ${currentUser?.id===user?.id?'disabled':''}> Active account</label></div><div id="userFormError" style="display:none;margin-top:12px;padding:10px 12px;border-radius:9px;background:#fff0f0;color:#b42318"></div><div style="display:flex;justify-content:flex-end;gap:10px;margin-top:20px"><button type="button" class="secondary" id="cancelUser">Cancel</button><button class="primary green-btn">${user?'Save changes':'Create user'}</button></div></form></div>`;
    root.querySelector('#closeUser').onclick = closeForm; root.querySelector('#cancelUser').onclick = closeForm; root.querySelector('#userForm').onsubmit = save;
  };
  const save = async event => {
    event.preventDefault();
    const form = event.currentTarget; const data = Object.fromEntries(new FormData(form));
    const error = root.querySelector('#userFormError'); const button = form.querySelector('button[type="submit"]');
    button.disabled = true; button.textContent = 'Saving…'; error.style.display='none';
    try {
      if (editing) {
        await invoke('user_update', {request:{id:editing.id, username:data.username, role:data.role, employeeId:data.employee_id || null, active:form.active.checked}});
      } else {
        await invoke('user_create', {request:{id:crypto.randomUUID(), username:data.username, password:data.password, role:data.role, employeeId:data.employee_id || null}});
      }
      users = await invoke('users_list'); closeForm(); renderRows(); showMessage(editing?'User updated successfully.':'User created successfully. The new user must change the temporary password on first login.');
    } catch (e) { error.textContent = String(e).replace(/^Error:\s*/,'') || 'Could not save user.'; error.style.display='block'; button.disabled=false; button.textContent=editing?'Save changes':'Create user'; }
  };
  const closeForm = () => { root.querySelector('#userModal').innerHTML=''; editing=null; };
  const showMessage = message => { const el=root.querySelector('#userMessage'); if(el){el.textContent=message;setTimeout(()=>{if(el.textContent===message)el.textContent=''},5000)} };
  try { users = await invoke('users_list'); employees = await invoke('employees_list'); draw(); } catch (e) { root.innerHTML=`<section class="content"><div class="panel"><h3>Could not load users</h3><p>${esc(e)}</p></div></section>`; }
}
