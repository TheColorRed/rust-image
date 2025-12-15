import { LayersPanel } from '@/client/components/panels/layers';
import { PropertiesPanel } from '@/client/components/panels/properties';
import { cn } from '@/client/lib/util';

export function Panels() {
  return (
    <div
      className={cn(
        'bg-dark',
        'flex h-full flex-col space-y-2 overflow-y-auto border-l border-white/30 p-2',
        '[&>div:not(:last-child)]:border-b [&>div:not(:last-child)]:border-white/30 [&>div:not(:last-child)]:pb-2',
        '[&>div]:bg-default [&>div]:rounded [&>div]:border [&>div]:border-white/30 [&>div]:p-4',
      )}
    >
      <div>
        <PropertiesPanel />
      </div>
      <div className="flex-1">
        <LayersPanel />
      </div>
    </div>
  );
}
