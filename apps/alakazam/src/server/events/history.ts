import { AbraHistory, AbraHistoryEntry } from '@alakazam/history';
import { ipcMain } from 'electron';

const history: AbraHistory[] = [];
/**
 * Adds a history entry for a given project.
 * @param projectId The ID of the project.
 * @param entry The history entry to add.
 */
export function addHistoryEntry(projectId: string, entry: AbraHistoryEntry) {
  const projectHistory = history.find(h => h.projectId === projectId);
  if (projectHistory) {
    projectHistory.add(entry);
  } else {
    const newHistory = new global.alakazamHistory.AbraHistory(projectId);
    newHistory.add(entry);
    history.push(newHistory);
  }

  for (const h of history) {
    console.log(`History for project ${h.projectId}: ${h.length} entries`);
  }
}
/**
 * Goes to a specific history entry for a given project.
 * @param projectId The ID of the project.
 * @param index The index of the history entry to go to.
 */
export function getHistoryEntry(projectId: string, index: number) {
  const projectHistory = history.find(h => h.projectId === projectId);
  if (projectHistory) {
    return projectHistory.get(index);
  }
}

ipcMain.handle('history:add', (event, item: History) => {});
