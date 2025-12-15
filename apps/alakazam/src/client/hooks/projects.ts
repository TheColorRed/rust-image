import { ProjectMetadata } from 'bindings';
import { useCallback, useEffect, useState } from 'react';

export function useProjects() {
  const [projects, setProjects] = useState<string[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [layers, setLayers] = useState<Record<string, any>>({});

  useEffect(() => {
    // Get initial list of projects
    window.alakazam.projects.getProjectMetadata().then(metadata => {
      metadata = [metadata].flat() as ProjectMetadata[];
      const projectIds = metadata.map(meta => meta.id);
      setProjects(projectIds);
      setActiveProjectId(metadata[0]?.id ?? null);
    });

    const onNewProject = (projectId: string) => {
      setProjects(prev => {
        if (prev.includes(projectId)) return prev;
        return [...prev, projectId];
      });
      setActiveProjectId(projectId);
    };

    const onCloseProject = (projectId: string) => {
      setProjects(prev => prev.filter(id => id !== projectId));
      setActiveProjectId(prev => (prev === projectId ? null : prev));
    };

    window.alakazam.onNewProject(onNewProject);
    window.alakazam.onCloseProject(onCloseProject);

    return () => {
      // ipcMain.off('new-project', onNewProject);
    };
  }, []);

  const open = useCallback(async () => {
    window.alakazam.openFileDialog();
  }, []);

  return {
    open,
    /** A list of project IDs. */
    projects,
    setProjects,
    activeProjectId,
    setActiveProjectId,
    layers,
  };
}
