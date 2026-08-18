import { mountUserManagement } from './modules/users.js';

const invoke = globalThis.__TAURI__?.core?.invoke;
let open = false;

function session() {
  try { return JSON.parse(sessionStorage.getItem('natra.session.v1') || 'null'); } catch { return null; }
}

function install() {
  const user = session();
  const nav = document.querySelector('.sidebar nav');
  const page = document.querySelector('#page');
  if (!nav || !page || user?.role !== 'HR Admin') return;
  let button = nav.querySelector('[data-page="User Management"]');
  if (!button) {
    button = document.createElement('button');
    button.className = 'nav-item';
    button.dataset.page = 'User Management';
    button.innerHTML = '<i>◌</i>User Management';
    nav.appendChild(button);
  }
  button.classList.toggle('active', open);
  button.onclick = async () => {
    open = true;
    document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
    button.classList.add('active');
    const title = document.querySelector('.topbar h1');
    if (title) title.textContent = 'User Management';
    await mountUserManagement(page, user);
  };
  if (!open) return;
  const current = document.querySelector('.topbar h1')?.textContent;
  if (current !== 'User Management') open = false;
}

const observer = new MutationObserver(() => install());
observer.observe(document.documentElement, {childList:true,subtree:true});
window.addEventListener('beforeunload', () => observer.disconnect());
setInterval(() => { if (!session()) open = false; install(); }, 1000);
