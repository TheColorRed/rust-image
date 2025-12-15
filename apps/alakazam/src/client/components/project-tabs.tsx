import { AppContext } from '@/client/app';
import { Project } from '@/client/components/project';
import { TabActiveChange, TabClose, TabItem, TabLabel, Tabs } from '@/client/ui/tabs';
import { useCallback, useContext, useEffect, useState } from 'react';

export function ProjectTabs() {
  const { projects: appProjects } = useContext(AppContext);
  const [tabLabels, setTabLabels] = useState<string[]>([]);

  useEffect(() => {
    window.alakazam.projects.getProjectMetadata().then(metadata => {
      const labels = [metadata].flat().map(m => m?.name ?? 'Untitled');
      setTabLabels(labels);
    });
  }, [appProjects.projects]);

  const handleClose = useCallback<TabClose>(
    (_, id) => window.alakazam.projects.closeProject(id as string),
    [appProjects],
  );

  const onActiveTabChange = useCallback<TabActiveChange>(
    (_, id) => {
      if (typeof id == 'string' || id === null) appProjects.setActiveProjectId(id);
    },
    [appProjects],
  );

  return (
    <div className="w-full">
      <Tabs
        activeId={appProjects.activeProjectId}
        closeable
        onActiveTabChange={onActiveTabChange}
        className="bg-dark"
        onClose={handleClose}
      >
        {appProjects.projects.map((project, index) => (
          <TabItem key={project} id={project}>
            <TabLabel>{tabLabels[index]}</TabLabel>
          </TabItem>
        ))}
      </Tabs>
      <Project />
    </div>
  );
}
