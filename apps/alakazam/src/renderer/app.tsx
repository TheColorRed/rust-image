import { useFiles } from '@/hooks/file';
import { useProjects } from '@/hooks/projects';
// import { addImage, numberOfProjects } from '@/lib/projects';
import { TitleBar } from '@/components/title-bar';
import { cursor$, mouseMove$ } from '@/events/body';
import Projects from '@/pages/projects';
import Welcome from '@/pages/welcome';
import { createContext, useEffect, useMemo, useRef, useState } from 'react';
import { filter } from 'rxjs';

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
  const appCursorRef = useRef<HTMLDivElement>(null);
  const currentCursor = useRef<string>('default');

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

  useEffect(() => {
    const cursorSubscription = cursor$.subscribe(({ cursor, origin, size }) => {
      if (!appCursorRef.current) return;
      if (cursor === 'default') {
        document.body.style.cursor = cursor;
        appCursorRef.current.style.backgroundImage = '';
        currentCursor.current = 'default';
        return;
      }
      size = Array.isArray(size) ? Math.max(size[0], size[1]) : size;
      appCursorRef.current.style.width = `${size}px`;
      appCursorRef.current.style.height = `${size}px`;
      appCursorRef.current.style.marginLeft = origin ? `-${origin[0]}px` : `-${size / 2}px`;
      appCursorRef.current.style.marginTop = origin ? `-${origin[1]}px` : `-${size / 2}px`;
      appCursorRef.current.style.backgroundImage = `url('${cursor}')`;
      appCursorRef.current.style.backgroundSize = 'contain';
      appCursorRef.current.style.backgroundRepeat = 'no-repeat';
      document.body.style.cursor = 'none';
      currentCursor.current = cursor;
    });
    const mouseMoveSubscription = mouseMove$.pipe(filter(() => currentCursor.current !== 'default')).subscribe(evt => {
      if (!appCursorRef.current) return;
      appCursorRef.current.style.left = `${evt.clientX}px`;
      appCursorRef.current.style.top = `${evt.clientY}px`;
    });
    return () => {
      cursorSubscription.unsubscribe();
      mouseMoveSubscription.unsubscribe();
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
    <>
      <div ref={appCursorRef} className="pointer-events-none absolute z-20" />
      <div className="bg-default flex h-screen flex-col text-white">
        <AppContext.Provider value={contextValue}>
          <TitleBar />
          <div className="flex-1 overflow-hidden">{projects.projects.length === 0 ? <Welcome /> : <Projects />}</div>
        </AppContext.Provider>
      </div>
    </>
  );
}
