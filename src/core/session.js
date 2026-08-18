const KEY='natra.session.v1';
let memory=null;
export function setSession(user){ memory=user; sessionStorage.setItem(KEY,JSON.stringify(user)); }
export function getSession(){ if(memory)return memory; try{return memory=JSON.parse(sessionStorage.getItem(KEY)||'null')}catch{return null} }
export function clearSession(){ memory=null; sessionStorage.removeItem(KEY); }
export function requireRole(role){ const s=getSession(); if(!s||s.role!==role) throw new Error('UNAUTHORIZED'); return s; }
export function canAccess(role, page){ const map={ 'HR Admin':new Set(['Dashboard','Employees','Attendance','Leave','Payroll','Documents','Reports']), Employee:new Set(['Dashboard','Attendance','Leave','Documents']) }; return !!map[role]?.has(page); }
