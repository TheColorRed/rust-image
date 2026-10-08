import { cn } from '@/lib/util';
import { ActiveColor, activeColorState, activeColorState$, colors$ } from '@/state/color';
import { cva } from 'class-variance-authority';
import { useEffect, useState } from 'react';

const activeContainer = cva('box-border', {
  variants: {
    layout: {
      vertical: 'flex flex-col gap-1',
      horizontal: 'flex h-10 min-h-10 flex-1 cursor-pointer items-stretch flex-row gap-1 p-1',
      overlap: 'relative aspect-square w-full',
    },
  },
});

const colorBox = cva('cursor-pointer box-border', {
  variants: {
    layout: {
      vertical: 'h-8 w-full',
      horizontal: 'flex flex-1 w-full rounded p-1',
      overlap: ' absolute aspect-square w-2/3',
    },
    active: {
      true: 'bg-white/20',
      false: '',
    },
    isFgOverlap: {
      true: 'top-0 left-0 z-10',
      false: '',
    },
    isBgOverlap: {
      true: 'bottom-0 right-0',
      false: '',
    },
  },
});

export function ActiveColors({ layout }: { layout?: 'vertical' | 'horizontal' | 'overlap' }) {
  const [fgColor, setFgColor] = useState('#ffffff');
  const [bgColor, setBgColor] = useState('#000000');
  const [activeColor, setActiveColor] = useState<ActiveColor>('fg');

  useEffect(() => {
    const colors = colors$.subscribe(({ fg, bg }) => [setFgColor(fg.color), setBgColor(bg.color)]);
    const activeColorSub = activeColorState$.subscribe(setActiveColor);
    return () => {
      activeColorSub.unsubscribe();
      colors.unsubscribe();
    };
  }, []);

  return (
    <div className={cn(activeContainer({ layout }))}>
      <div
        className={cn(
          colorBox({ layout, active: activeColor === 'fg', isFgOverlap: layout === 'overlap' }),
          layout !== 'overlap' && 'flex',
        )}
        onClick={() => activeColorState.next('fg')}
      >
        <div className="h-full w-full border border-black" style={{ backgroundColor: fgColor }} />
      </div>
      <div
        className={cn(
          colorBox({ layout, active: activeColor === 'bg', isBgOverlap: layout === 'overlap' }),
          layout !== 'overlap' && 'flex',
        )}
        onClick={() => activeColorState.next('bg')}
      >
        <div className="h-full w-full border border-black" style={{ backgroundColor: bgColor }} />
      </div>
    </div>
  );
}
