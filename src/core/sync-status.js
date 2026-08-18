export const SYNC_STATUS={OFFLINE:'offline',CONNECTING:'connecting',SYNCING:'syncing',CONNECTED:'connected',ERROR:'error'};
let current={state:SYNC_STATUS.OFFLINE,pending:0,lastError:null,lastSync:null};
const listeners=new Set();
export function getSyncStatus(){return current}
export function setSyncStatus(patch){current={...current,...patch};listeners.forEach(fn=>fn(current));}
export function subscribeSyncStatus(fn){listeners.add(fn);fn(current);return()=>listeners.delete(fn)}
