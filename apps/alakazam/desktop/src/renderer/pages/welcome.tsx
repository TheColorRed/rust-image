import { AppContext } from '@/app';
import { runCommand } from '@/data/commands';
import { Button } from '@/ui/button';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faImage } from '@fortawesome/sharp-light-svg-icons/faImage';
import { useContext } from 'react';

export default function Welcome() {
  let { projects } = useContext(AppContext);

  return (
    <div className="flex h-full items-center justify-center">
      <div className="grid h-[500px] w-[800px] grid-cols-2 gap-4">
        <div className="flex flex-col border-r border-white/30 p-4">
          <h1 className="text-4xl">Alakazam</h1>
          <div className="mt-8 flex flex-col gap-2">
            <Button variant="ghost-hover" size="lg" className="justify-between" onClick={() => runCommand('open-file')}>
              <div>Open File</div>
              <FontAwesomeIcon icon={faImage} size="lg" />
            </Button>
            {/* <Button variant="ghost-hover" size="lg" className="justify-between" onClick={() => ipc.createNewProject()}>
              <div>New Project</div>
              <FontAwesomeIcon icon={faLayerGroup} size="lg" />
            </Button> */}
          </div>
        </div>
        <div>
          <h2 className="text-2xl">Recent</h2>
        </div>
      </div>
    </div>
  );
}
