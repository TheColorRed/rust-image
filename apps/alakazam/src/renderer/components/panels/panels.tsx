import { ColorsPanel } from '@/components/panels/colors/panel';
import { LayersPanel } from '@/components/panels/layers/panel';
import { PropertiesPanel } from '@/components/panels/properties/properties';
import { cn } from '@/lib/util';
import { Button } from '@/ui/button';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faChevronRight } from '@fortawesome/sharp-light-svg-icons';
import { ReactNode, useCallback, useState } from 'react';

function PanelSection({
  title,
  children,
  className,
  collapsed,
}: {
  title: string;
  children: ReactNode;
  className?: string;
  collapsed?: boolean;
}) {
  const [isOpen, setIsOpen] = useState(!(collapsed ?? false));

  const toggleOpen = useCallback(() => {
    setIsOpen(prev => !prev);
  }, []);

  return (
    <div className={cn('flex flex-col', className)}>
      <Button variant="ghost" className="mb-2 px-0 text-lg font-medium" onClick={toggleOpen}>
        <FontAwesomeIcon icon={isOpen ? faChevronDown : faChevronRight} className="mr-2" />
        {title}
      </Button>
      {isOpen && <div className="min-h-0 flex-1">{children}</div>}
    </div>
  );
}

export function Panels() {
  return (
    <div
      className={cn(
        'bg-dark',
        'flex h-full flex-col space-y-2 overflow-y-auto border-l border-white/30 p-2',
        '[&>div:not(:last-child)]:border-b [&>div:not(:last-child)]:border-white/30 [&>div:not(:last-child)]:pb-2',
        '[&>div]:bg-default [&>div]:rounded [&>div]:border [&>div]:border-white/30',
      )}
    >
      <PanelSection title="Colors" className="[&_button]:px-4 [&_button]:pt-4">
        <ColorsPanel />
      </PanelSection>
      <PanelSection title="Properties" collapsed className="p-4">
        <PropertiesPanel />
      </PanelSection>
      <PanelSection title="Layers" className="flex-1 p-4">
        <LayersPanel />
      </PanelSection>
    </div>
  );
}
