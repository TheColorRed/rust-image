import { Panels } from '@/components/panels/panels';
import { ProjectTabs } from '@/components/project-tabs';
import { StatusBar } from '@/components/status-bar';
import { ToolBar } from '@/components/tool-bar';
import { TopToolBar } from '@/components/top-tool-bar';

export default function Projects() {
  return (
    <>
      <div className="flex h-full w-full flex-col overflow-hidden">
        <div className="flex min-h-0 w-full grow">
          <div className="min-w-14">
            <ToolBar />
          </div>
          <div className="flex min-h-0 min-w-0 grow flex-col">
            <TopToolBar />
            <div className="flex min-h-0 min-w-0 grow overflow-hidden">
              {/* Make the image area flex-grow so it takes remaining space */}
              <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
                <ProjectTabs />
              </div>
              {/* Fix panels on the right to a fixed width and prevent it from shrinking */}
              <div className="border-default min-h-0 w-100 min-w-0 shrink-0 overflow-y-auto border-l">
                <Panels />
              </div>
            </div>
          </div>
        </div>
        <div className="shrink-0">
          <StatusBar />
        </div>
      </div>
    </>
  );
}
