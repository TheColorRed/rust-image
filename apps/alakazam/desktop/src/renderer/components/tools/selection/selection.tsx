import { AppContext } from '@/app';
import { EllipseSelectionArea } from '@/components/tools/selection/ellipse';
import { LassoSelectionArea } from '@/components/tools/selection/lasso';
import { MagneticLassoSelectionArea } from '@/components/tools/selection/magnetic-lasso';
import { RectSelectionArea } from '@/components/tools/selection/rect';
import { SelectionArea } from '@/components/tools/selection/selection-area';
import { canvasGizmos, canvasMouse$, projectCursor } from '@/events/project';
import { useNumericInputValidation } from '@/hooks/numeric-input-validation';
import { toSvgCursor } from '@/lib/cursor';
import { useNumericInput } from '@/lib/util';
import { Input } from '@/ui/input';
import { Option } from '@/ui/option';
import { Select } from '@/ui/select';
import { Separator } from '@/ui/separator';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCircle, faLasso, faPlus, faSquare } from '@fortawesome/sharp-light-svg-icons';
import { useContext, useEffect, useMemo, useState } from 'react';
import { filter } from 'rxjs/operators';

/**
 * Draws a polygon shape on the canvas context.
 * @param ctx The canvas rendering context.
 * @param points The array of points defining the polygon shape.
 */
const drawPoints = (ctx: CanvasRenderingContext2D, points: [number, number][]) => {
  if (points.length < 2) return;
  ctx.beginPath();
  ctx.moveTo(points[0][0], points[0][1]);
  for (let i = 1; i < points.length; i++) ctx.lineTo(points[i][0], points[i][1]);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
};
/**
 * Draws a line where the feathered selection would be.
 * @param ctx The canvas rendering context.
 * @param areaPoints The points defining the selection area.
 * @param feather The feather amount in pixels.
 */
const drawFeatherLine = (ctx: CanvasRenderingContext2D, areaPoints: [number, number][], feather: number) => {
  // Draw a line that is inset within the selection area to represent feathering
  ctx.strokeStyle = 'rgba(255, 0, 255, 0.5)';
  ctx.fillStyle = 'rgba(0, 0, 0, 0.0)';
  ctx.lineDashOffset = 0;
  ctx.lineWidth = 1;
  ctx.setLineDash([8, 8]);

  const points = SelectionArea.computeInsetPolygon(areaPoints, feather);

  drawPoints(ctx, points);

  ctx.strokeStyle = 'rgba(0, 0, 0, 0.5)';
  ctx.fillStyle = 'rgba(0, 0, 0, 0.0)';
  ctx.lineDashOffset = 8;

  drawPoints(ctx, points);
};
/**
 * Draw call for selection tool gizmos.
 * @param ctx The canvas rendering context.
 * @param areaPoints The main selection area points.
 * @param tmpAreaPoints The temporary selection area points (for add/subtract).
 */
const drawCall = (
  ctx: CanvasRenderingContext2D,
  areaPoints: [number, number][],
  tmpAreaPoints: [number, number][],
  feather: number,
) => {
  const linePattern = [8, 8];
  // Clear previous gizmos
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

  // Setup non-changing and initial context state
  ctx.lineDashOffset = 0;
  ctx.lineWidth = 1;
  ctx.setLineDash(linePattern);

  // Draw white filled shape with some opacity
  ctx.strokeStyle = 'rgba(255, 255, 255, 1)';
  // Keep the area transparent. This prevents flickering on update
  ctx.fillStyle = 'rgba(255, 255, 255, 0.0)';

  // First pass: draw white outline and fill to see on dark areas
  if (areaPoints.length > 2) drawPoints(ctx, areaPoints);
  if (tmpAreaPoints.length > 2) drawPoints(ctx, tmpAreaPoints);

  // Second pass: draw black intermediate outline to see on light areas
  ctx.strokeStyle = 'rgba(0, 0, 0, 1)';
  ctx.fillStyle = 'rgba(0, 0, 0, 0.0)';
  ctx.lineDashOffset = 8;
  if (areaPoints.length > 2) drawPoints(ctx, areaPoints);
  if (tmpAreaPoints.length > 2) drawPoints(ctx, tmpAreaPoints);

  if (feather > 0) {
    if (areaPoints.length >= 3) drawFeatherLine(ctx, areaPoints, feather);
    if (tmpAreaPoints.length >= 3) drawFeatherLine(ctx, tmpAreaPoints, feather);
  }
};

export function SelectionTool() {
  const { projects } = useContext(AppContext);
  const [shape, setShape] = useState<'rect' | 'circle' | 'lasso' | 'magnetic-lasso'>('rect');
  const [style, setStyle] = useState<'normal' | 'ratio' | 'pixels'>('normal');
  const [feather, setFeather] = useNumericInput(0, { min: 0, max: 1000, step: 1 });
  const { onKeyDown, onPaste } = useNumericInputValidation();

  const area = useMemo(() => {
    if (shape === 'rect') return new RectSelectionArea();
    else if (shape === 'circle') return new EllipseSelectionArea();
    else if (shape === 'lasso') return new LassoSelectionArea();
    else if (shape === 'magnetic-lasso') return new MagneticLassoSelectionArea();
    return new RectSelectionArea();
  }, [shape]);

  useEffect(() => {
    const cursor = toSvgCursor(faPlus);
    projectCursor.next({ cursor, origin: [10, 10], size: 20 });
    // canvasGizmos.next({ action: 'clear' });
  }, []);

  useEffect(() => {
    const projectId = projects.activeProjectId;
    if (!projectId) return;
    window.tools.selection.setFeather(projectId, feather.num);
  }, [feather]);

  // Listen to canvas mouse events
  // Subscribe/unsubscribe when `shape` changes to avoid stale closures
  useEffect(() => {
    /** Handle mouse down events for selection tool. */
    const mouseDown = canvasMouse$.pipe(filter(e => e.type === 'down')).subscribe(({ x, y, event }) => {
      area.setStartPoint(x, y);

      // Determine merge type based on modifier keys
      const merge = event.shiftKey ? 'add' : event.ctrlKey ? 'subtract' : 'none';
      area.setMergeType(merge);
      if (merge === 'none') area.clear('main');

      // Trigger the mouse down action
      area.mouseDown(x, y);
    });
    /** Handle mouse move events for selection tool. */
    const mouseMove = canvasMouse$.pipe(filter(e => e.type === 'move')).subscribe(({ x, y, isMouseDown }) => {
      area.setCurrentPoint(x, y);
      if (isMouseDown) area.mouseMove();
    });
    /** Handle mouse up events for selection tool. */
    const mouseUp = canvasMouse$.pipe(filter(e => e.type === 'up')).subscribe(({ x, y }) => {
      const projectId = projects.activeProjectId;
      if (!projectId) return;

      area.setEndPoint(x, y);
      area.mergePoints();
      area.clear('tmp');

      // If the selection has less than 3 points, it's invalid - clear it completely
      if (area.points.length < 3) {
        window.tools.selection.clear(projectId);
        area.clear('main');
        canvasGizmos.next({ action: 'clear' });
      } else {
        window.tools.selection.setArea(projectId, area.points);
        // Immediately redraw the merged area on mouse up so the user sees the result
        canvasGizmos.next({
          action: 'draw',
          callback: ctx => drawCall(ctx, area.points, area.tmpPoints, feather.num),
        });
      }
    });

    return () => {
      mouseDown.unsubscribe();
      mouseMove.unsubscribe();
      mouseUp.unsubscribe();
    };
  }, [shape, feather, area, projects]);

  // Draw rectangle/circle on the gizmos canvas
  useEffect(() => {
    let animationFrameId: number;
    const animation = () => {
      canvasGizmos.next({
        action: 'draw',
        callback: ctx => drawCall(ctx, area.points, area.tmpPoints, feather.num),
      });
      animationFrameId = requestAnimationFrame(animation);
    };
    animationFrameId = requestAnimationFrame(animation);
    return () => cancelAnimationFrame(animationFrameId);
  }, [feather.num, area]);

  return (
    <div className="flex w-full items-center gap-4">
      <div>
        <Select value={shape} onChange={setShape}>
          <Option value="rect">
            <FontAwesomeIcon icon={faSquare} /> Rectangle
          </Option>
          <Option value="circle">
            <FontAwesomeIcon icon={faCircle} /> Ellipse
          </Option>
          <Option value="lasso">
            <FontAwesomeIcon icon={faLasso} /> Lasso
          </Option>
          <Option value="magnetic-lasso">
            <FontAwesomeIcon icon={faLasso} /> Magnetic Lasso
          </Option>
        </Select>
      </div>
      <Separator direction="vertical" />
      <div className="flex w-34 items-center gap-2">
        Feather:
        <Input
          value={feather.input}
          onChange={setFeather}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
          selectFocus
          min={0}
          max={100}
          step={1}
        />
      </div>
      <Separator direction="vertical" />
      <div className="flex items-center gap-8">
        <div className="flex flex-1 items-center gap-2">
          Style:
          <Select value={style} onChange={setStyle} className="w-40">
            <Option value="normal">Normal</Option>
            <Option value="ratio">Fixed Ratio</Option>
            <Option value="pixels">Fixed Pixels</Option>
          </Select>
        </div>
        <div className="flex flex-1 items-center gap-2">
          Width:
          <Input className="w-16" disabled={style === 'normal'} />
        </div>
        <div className="flex flex-1 items-center gap-2">
          Height:
          <Input className="w-16" disabled={style === 'normal'} />
        </div>
      </div>
    </div>
  );
}
