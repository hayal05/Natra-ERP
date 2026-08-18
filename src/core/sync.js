/** Sync queue contract for the future Turso adapter.
 * Mutations are recorded locally first. When an authenticated cloud adapter is
 * supplied, pending operations can be uploaded and acknowledged safely.
 */
import { storage } from './storage.js';

const KEY = 'sync-queue';

export const syncQueue = {
  list() { return storage.read(KEY, []); },
  enqueue(operation) {
    const queue = this.list();
    queue.push({ id: crypto.randomUUID(), createdAt: new Date().toISOString(), attempts: 0, ...operation });
    storage.write(KEY, queue);
  },
  remove(id) { storage.write(KEY, this.list().filter(item => item.id !== id)); },
  clear() { storage.write(KEY, []); },
  pendingCount() { return this.list().length; }
};

export async function flushSyncQueue(adapter) {
  if (!adapter?.apply) return { synced: 0, pending: syncQueue.pendingCount() };
  let synced = 0;
  for (const operation of syncQueue.list()) {
    try { await adapter.apply(operation); syncQueue.remove(operation.id); synced++; }
    catch { break; }
  }
  return { synced, pending: syncQueue.pendingCount() };
}
