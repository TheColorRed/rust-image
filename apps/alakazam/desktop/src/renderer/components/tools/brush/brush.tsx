import { mouseWheel$ } from '@/events/body';
import { projectCursor } from '@/events/project';
import { useNumericInputValidation } from '@/hooks/numeric-input-validation';
import { toPngCursor } from '@/lib/cursor';
import { cn, useNumericInput } from '@/lib/util';
import { Input } from '@/ui/input';
import { useEffect } from 'react';
import { throttleTime } from 'rxjs';

export function BrushTool() {
  const [brushSize, setBrushSize] = useNumericInput(10, { min: 1, max: 5000, step: 1 });
  const { onKeyDown, onPaste } = useNumericInputValidation();

  useEffect(() => {
    window.gizmos.cursor.circleCursor(brushSize.num).then(imageData => {
      projectCursor.next({
        cursor: toPngCursor(imageData),
        origin: [brushSize.num / 2, brushSize.num / 2],
        size: brushSize.num,
      });
    });
  }, [brushSize]);

  useEffect(() => {
    const subscription = mouseWheel$.pipe(throttleTime(100)).subscribe(evt => {
      let amount = 50;
      if (evt.shiftKey) amount = 25;
      else if (evt.ctrlKey) amount = 10;
      else if (evt.altKey) amount = 1;

      amount = Math.max(1, Math.min(5000, amount));
      setBrushSize(brushSize.num + (evt.deltaY < 0 ? amount : -amount));
    });
    return () => subscription.unsubscribe();
  }, [brushSize]);

  return (
    <div
      className={cn(
        // Applied to self
        'flex items-center gap-4',
        // Applied to children
        '[&>div]:flex [&>div]:items-center [&>div]:gap-2',
      )}
    >
      <div>
        <div>Size:</div>
        <Input selectFocus onKeyDown={onKeyDown} onPaste={onPaste} value={brushSize.input} onChange={setBrushSize} />
      </div>
    </div>
  );
}
