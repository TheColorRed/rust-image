import { type EditablePhoto } from '@/src/lib/edit-source';
import { errorMessage } from '@/src/lib/error-message';
import { deleteMediaFromCache } from '@/src/lib/media-file';
import { createProject, deleteProject, listProjects, writeProject, type Project } from '@/src/lib/project-store';
import { Alert } from 'react-native';
import { BehaviorSubject } from 'rxjs';

/** Every saved project, newest first. They live in the app's files folder, so they are still here after a restart. */
export const projects = new BehaviorSubject<Project[]>([]);
export const projects$ = projects.asObservable();

/** Makes a project from a photo (Camera Roll or file picker) and adds it to `projects`. */
export async function addProject(photo: EditablePhoto): Promise<void> {
  try {
    const project = await createProject(photo.uri, photo.fileName ?? `photo-${Date.now()}.jpg`);
    projects.next([project, ...projects.value]);
    // A picker leaves its own temporary copy in the cache; the project has its own now.
    await deleteMediaFromCache(photo.uri);
  } catch (error) {
    Alert.alert('Could not add the image', errorMessage(error));
  }
}

/** Deletes the projects and takes them off the list. The photos they were made from are never touched. */
export async function removeProjects(ids: string[]): Promise<void> {
  projects.next(projects.value.filter(project => !ids.includes(project.id)));
  try {
    await Promise.all(ids.map(deleteProject));
  } catch (error) {
    Alert.alert('Could not delete a project', errorMessage(error));
  }
}

/** Fills `projects` from the files folder, so projects from an earlier run are still there. Call once at startup. */
export async function restoreProjects(): Promise<void> {
  try {
    const known = new Set(projects.value.map(project => project.id));
    const restored = (await listProjects()).filter(project => !known.has(project.id));
    projects.next([...projects.value, ...restored].sort((a, b) => b.createdAt - a.createdAt));
  } catch (error) {
    Alert.alert('Could not restore the projects', errorMessage(error));
  }
}

/** Saves a changed project and swaps it into `projects`, so the grid shows its new final image. */
export async function updateProject(project: Project): Promise<void> {
  projects.next(projects.value.map(existing => (existing.id === project.id ? project : existing)));
  await writeProject(project);
}
