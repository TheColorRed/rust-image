import { AppContext } from '@/app';
import { MoveToolOptions } from '@/components/tools/move';
import { SelectionTool } from '@/components/tools/selection';
import { useContext, useMemo } from 'react';

export function TopToolBar() {
  const { activeTool } = useContext(AppContext);

  const tool = useMemo(() => {
    switch (activeTool) {
      case 'move':
        return <MoveToolOptions />;
      case 'selection':
        return <SelectionTool />;
      default:
        return <div>No Options Available</div>;
    }
  }, [activeTool]);

  return <div className="flex h-16 max-h-16 w-full items-center border-b border-white/30 p-2">{tool}</div>;
}
