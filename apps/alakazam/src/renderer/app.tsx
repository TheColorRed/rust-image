import { useFiles } from '@/hooks/file';
import { useProjects } from '@/hooks/projects';
// import { addImage, numberOfProjects } from '@/lib/projects';
import { TitleBar } from '@/components/title-bar';
import Projects from '@/pages/projects';
import Welcome from '@/pages/welcome';
import { createContext, useEffect, useMemo, useRef, useState } from 'react';

export const AppContext = createContext({
  projects: {} as ReturnType<typeof useProjects>,
  activeTool: 'move',
  setActiveTool: (_tool: string) => {},
  ipc: {} as ReturnType<typeof useFiles>,
});

export default function App() {
  const [activeTool, setActiveTool] = useState('move');
  const ipc = useFiles();
  const projects = useProjects();
  const timeoutRef = useRef<NodeJS.Timeout>(null);

  useEffect(() => {
    let handler = ipc.on('fileOpened', event => {
      const { projectId } = event.detail;
      console.log('File opened:', projectId);
      projects.setProjects(prev => {
        if (prev.includes(projectId)) return prev;
        return [...prev, projectId];
      });
      clearTimeout(timeoutRef.current ?? undefined);
      timeoutRef.current = setTimeout(() => projects.setActiveProjectId(projectId), 150);
    });
    return () => {
      ipc.off('fileOpened', handler);
      clearTimeout(timeoutRef.current ?? undefined);
    };
  }, [ipc]);

  useEffect(() => {
    // Listen for console messages from dialogs for easier debugging.
    const unsubscribe = window.alakazam.onDialogConsoleMessage(message => {
      if (message.level === 1) console.log('[Dialog Console]', message.message);
      else if (message.level === 2) console.warn('[Dialog Console]', message.message);
      else if (message.level === 3) console.error('[Dialog Console]', message.message);
    });
    return () => {
      unsubscribe();
    };
  }, []);

  const contextValue = useMemo(
    () => ({
      projects,
      ipc,
      activeTool,
      setActiveTool,
    }),
    [projects, activeTool, ipc],
  );

  return (
    <div className="bg-default flex h-screen flex-col text-white">
      <AppContext.Provider value={contextValue}>
        <TitleBar />
        <div className="flex-1 overflow-hidden">{projects.projects.length === 0 ? <Welcome /> : <Projects />}</div>
      </AppContext.Provider>
    </div>
  );
}
