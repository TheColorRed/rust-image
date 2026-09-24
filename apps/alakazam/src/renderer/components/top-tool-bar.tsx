import { AppContext } from '@/app';
import { BrushTool } from '@/components/tools/brush/brush';
import { EyeDropperTool } from '@/components/tools/eye-dropper/eye-dropper';
import { GradientTool } from '@/components/tools/gradient/gradient';
import { MoveToolOptions } from '@/components/tools/move';
import { SelectionTool } from '@/components/tools/selection/selection';
import { useContext, useMemo } from 'react';

export function TopToolBar() {
  const { activeTool } = useContext(AppContext);

  const tool = useMemo(() => {
    switch (activeTool) {
      case 'move':
        return <MoveToolOptions />;
      case 'selection':
        return <SelectionTool />;
      case 'paint-brush':
        return <BrushTool />;
      case 'eye-dropper':
        return <EyeDropperTool />;
      case 'gradient':
        return <GradientTool />;
      default:
        return <div>No Options Available</div>;
    }
  }, [activeTool]);

  return <div className="flex h-16 max-h-16 w-full items-center border-b border-white/30 p-2">{tool}</div>;
}
