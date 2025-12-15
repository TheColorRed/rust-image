import { AppContext } from '@/client/app';
import { MoveToolOptions } from '@/client/components/tools/move';
import { useContext, useMemo } from 'react';

export function TopToolBar() {
  const { activeTool } = useContext(AppContext);

  const tool = useMemo(() => {
    switch (activeTool) {
      case 'move':
        return <MoveToolOptions />;
      default:
        return <div>No Options Available</div>;
    }
  }, [activeTool]);

  return <div className="flex h-10 w-full items-center border-b border-white/30 p-2">{tool}</div>;
}
