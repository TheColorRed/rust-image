import { AppContext } from '@/app';
import { canvasGizmos, canvasMouse$, projectCursor } from '@/events/project';
import { toSvgCursor } from '@/lib/cursor';
import { backgroundColor$, foregroundColor$ } from '@/state/color';
import { Option } from '@/ui/option';
import { Select } from '@/ui/select';
import Slider from '@/ui/slider';
import { faPlus } from '@fortawesome/sharp-light-svg-icons';
import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import { filter } from 'rxjs/operators';

export function GradientTool() {
  const [gradientType, setGradientType] = useState<'linear' | 'radial'>('linear');
  const [opacity, setOpacity] = useState(100);
  const [fgColor, setFgColor] = useState('#ffffff');
  const [bgColor, setBgColor] = useState('#000000');
  const lineRef = useRef<{ start: [number, number]; end: [number, number] } | null>(null);
  const [line, setLine] = useState<{ start: [number, number]; end: [number, number] } | null>(null);
  const dragTargetRef = useRef<'start' | 'end' | null>(null);
  const { projects } = useContext(AppContext);
  const HANDLE_RADIUS = 10;

  const setCrosshairCursor = () => {
    const cursor = toSvgCursor(faPlus);
    projectCursor.next({ cursor, origin: [10, 10], size: 20 });
  };

  const setGrabCursor = (nativeCursor: string) => {
    projectCursor.next({ cursor: nativeCursor, isNativeCursor: true });
  };

  useEffect(() => {
    setCrosshairCursor();
  }, []);

  // Subscribe to foreground and background colors
  useEffect(() => {
    const fgSub = foregroundColor$.subscribe(c => setFgColor(c.color));
    const bgSub = backgroundColor$.subscribe(c => setBgColor(c.color));
    return () => {
      fgSub.unsubscribe();
      bgSub.unsubscribe();
    };
  }, []);

  // Capture line from canvas interactions.
  useEffect(() => {
    const hitTestHandle = (x: number, y: number) => {
      const current = lineRef.current;
      if (!current) return null;
      const dist = (ax: number, ay: number, bx: number, by: number) => Math.hypot(ax - bx, ay - by);
      if (dist(x, y, current.start[0], current.start[1]) <= HANDLE_RADIUS) return 'start' as const;
      if (dist(x, y, current.end[0], current.end[1]) <= HANDLE_RADIUS) return 'end' as const;
      return null;
    };

    const downSub = canvasMouse$.pipe(filter(e => e.type === 'down')).subscribe(({ x, y }) => {
      const handle = hitTestHandle(x, y);
      if (handle) {
        dragTargetRef.current = handle;
        const base = lineRef.current!;
        const nextLine = handle === 'start' ? { start: [x, y], end: base.end } : { start: base.start, end: [x, y] };
        lineRef.current = nextLine;
        setLine(nextLine);
        setGrabCursor('grabbing');
        return;
      }

      dragTargetRef.current = 'end';
      const nextLine = { start: [x, y] as [number, number], end: [x, y] as [number, number] };
      lineRef.current = nextLine;
      setLine(nextLine);
      setGrabCursor('grabbing');
    });

    const moveSub = canvasMouse$.pipe(filter(e => e.type === 'move' && e.isMouseDown)).subscribe(({ x, y }) => {
      if (!lineRef.current) return;
      if (dragTargetRef.current === 'start') {
        const nextLine = { start: [x, y] as [number, number], end: lineRef.current.end };
        lineRef.current = nextLine;
        setLine(nextLine);
      } else if (dragTargetRef.current === 'end') {
        const nextLine = { start: lineRef.current.start, end: [x, y] as [number, number] };
        lineRef.current = nextLine;
        setLine(nextLine);
      }
    });

    // Hover feedback: show grab hand when over a handle (and not dragging), crosshair otherwise.
    const hoverSub = canvasMouse$.pipe(filter(e => e.type === 'move' && !e.isMouseDown)).subscribe(({ x, y }) => {
      if (dragTargetRef.current) return;
      const handle = hitTestHandle(x, y);
      if (handle) {
        setGrabCursor('grab');
      } else {
        setCrosshairCursor();
      }
    });

    const upSub = canvasMouse$.pipe(filter(e => e.type === 'up')).subscribe(({ x, y }) => {
      if (!lineRef.current) return;

      let nextLine: { start: [number, number]; end: [number, number] };
      if (dragTargetRef.current === 'start') {
        // Preserve the dragged start; keep the existing end.
        nextLine = { start: lineRef.current.start, end: lineRef.current.end };
      } else if (dragTargetRef.current === 'end') {
        // Preserve the dragged end; keep the existing start.
        nextLine = { start: lineRef.current.start, end: lineRef.current.end };
      } else {
        // New line creation path.
        nextLine = { start: lineRef.current.start, end: [x, y] as [number, number] };
      }

      lineRef.current = nextLine;
      setLine(nextLine);
      dragTargetRef.current = null;
      setCrosshairCursor();

      const projectId = projects.activeProjectId;
      if (!projectId) return;
      window.tools.gradient.apply(projectId, {
        start: nextLine.start,
        end: nextLine.end,
        type: gradientType,
        opacity: opacity / 100, // Convert 0-100 to 0-1
        colorStart: fgColor,
        colorEnd: bgColor,
      });
    });

    return () => {
      downSub.unsubscribe();
      moveSub.unsubscribe();
      hoverSub.unsubscribe();
      upSub.unsubscribe();
    };
  }, [projects, gradientType, opacity, fgColor, bgColor]);

  // Clear the line when ESC is pressed.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      lineRef.current = null;
      setLine(null);
      dragTargetRef.current = null;
      canvasGizmos.next({ action: 'clear' });
      setCrosshairCursor();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  // Draw the helper line on the gizmos canvas.
  useEffect(() => {
    let animationFrameId: number;
    const draw = () => {
      canvasGizmos.next({
        action: 'draw',
        callback: ctx => {
          ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
          const currentLine = lineRef.current;
          if (!currentLine) return;
          const { start, end } = currentLine;
          if (start[0] === end[0] && start[1] === end[1]) return;

          ctx.lineWidth = 2;
          ctx.strokeStyle = 'rgba(255, 255, 255, 0.9)';
          ctx.setLineDash([6, 6]);
          ctx.beginPath();
          ctx.moveTo(start[0], start[1]);
          ctx.lineTo(end[0], end[1]);
          ctx.stroke();

          ctx.setLineDash([]);
          ctx.fillStyle = 'rgba(255, 255, 255, 0.95)';
          ctx.strokeStyle = 'rgba(0, 0, 0, 0.6)';
          [start, end].forEach(([x, y]) => {
            ctx.beginPath();
            ctx.arc(x, y, HANDLE_RADIUS, 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          });
        },
      });
      animationFrameId = requestAnimationFrame(draw);
    };
    animationFrameId = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animationFrameId);
  }, []);

  const computedAngle = useMemo(() => {
    if (!line) return null;
    const [sx, sy] = line.start;
    const [ex, ey] = line.end;
    if (sx === ex && sy === ey) return null;
    const radians = Math.atan2(ey - sy, ex - sx);
    const degrees = (radians * 180) / Math.PI;
    const normalized = (degrees + 360) % 360;
    return Number(normalized.toFixed(1));
  }, [line]);

  const lineLength = useMemo(() => {
    if (!line) return null;
    const [sx, sy] = line.start;
    const [ex, ey] = line.end;
    const distance = Math.hypot(ex - sx, ey - sy);
    return Number(distance.toFixed(1));
  }, [line]);

  return (
    <div className="flex w-full items-center gap-6">
      <div className="flex items-center gap-2">
        <span>Type:</span>
        <Select value={gradientType} onChange={setGradientType} className="w-32">
          <Option value="linear">Linear</Option>
          <Option value="radial">Radial</Option>
        </Select>
      </div>
      <div className="flex items-center gap-3">
        <span className="w-16">Opacity</span>
        <div className="w-40">
          <Slider min={0} max={100} value={opacity} step={1} onChange={setOpacity} />
        </div>
        <span className="text-sm text-white/60">{opacity}%</span>
      </div>
      <div className="flex items-center gap-3 text-sm text-white/80">
        <span>Angle: {computedAngle !== null ? `${computedAngle}°` : '—'}</span>
        <span>Length: {lineLength !== null ? `${lineLength}px` : '—'}</span>
      </div>
    </div>
  );
}
