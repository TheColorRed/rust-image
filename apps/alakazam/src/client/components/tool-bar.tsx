import { AppContext } from '@/client/app';
import { cn } from '@/client/lib/util';
import { Button } from '@/client/ui/button';
import { Tooltip } from '@/client/ui/tooltip';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faArrowsUpDownLeftRight,
  faEyeDropper,
  faPaintBrushFine,
  faPencilMechanical,
  faSquareDashed,
  IconDefinition,
} from '@fortawesome/sharp-light-svg-icons';
import { createContext, useContext, useState } from 'react';

const ToolBarContext = createContext({
  selectedTool: null as string | null,
  setSelectedTool: (_tool: string | null) => {},
});

function Tool({ icon, toolName, tooltip }: { icon: IconDefinition; toolName: string; tooltip: string }) {
  const { activeTool, setActiveTool } = useContext(AppContext);

  return (
    <Tooltip content={tooltip} position="right">
      <Button
        variant="icon"
        aspect="square"
        className={cn({
          'bg-dark': activeTool === toolName,
          'hover:bg-dark': activeTool === toolName,
        })}
        onClick={() => setActiveTool(toolName)}
      >
        <FontAwesomeIcon icon={icon} />
      </Button>
    </Tooltip>
  );
}

export function ToolBar() {
  const [selectedTool, setSelectedTool] = useState<string | null>(null);

  return (
    <ToolBarContext.Provider value={{ selectedTool, setSelectedTool }}>
      <div className="flex h-full w-full flex-col gap-1 border-r border-white/30 p-2 text-2xl">
        <Tool tooltip="Move tool" toolName="move" icon={faArrowsUpDownLeftRight} />
        <Tool tooltip="Select tool" toolName="select" icon={faSquareDashed} />
        <Tool tooltip="Eyedropper tool" toolName="eye-dropper" icon={faEyeDropper} />
        <Tool tooltip="Paint brush tool" toolName="paint-brush" icon={faPaintBrushFine} />
        <Tool tooltip="Pencil tool" toolName="pencil" icon={faPencilMechanical} />
      </div>
    </ToolBarContext.Provider>
  );
}
