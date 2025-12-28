import { AppContext } from '@/app';
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
import * as polygonClipping from 'polygon-clipping';
import { useContext, useEffect, useRef, useState } from 'react';
import { filter } from 'rxjs/operators';

// @ts-ignore: import abra from '@alakazam/abra';
import ClipperLib from 'clipper-lib';

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
 * Compute signed area of a ring (positive if CCW).
 * @param ring The array of points defining the ring.
 * @returns The signed area of the ring.
 */
const signedArea = (ring: [number, number][]) => {
  let a = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x1, y1] = ring[i];
    const [x2, y2] = ring[(i + 1) % ring.length];
    a += x1 * y2 - x2 * y1;
  }
  return a / 2;
};
/**
 * Pick the largest outer ring (by absolute area) from a polygon-clipping result and return it as a simple ring.
 * @param result The polygon-clipping result.
 * @returns The largest outer ring as an array of points.
 */
const pickLargestOuterRing = (result: any): [number, number][] => {
  if (!result || result.length === 0) return [];
  // result is an array of polygons; each polygon is an array of rings. Outer ring is index 0.
  let best: [number, number][] = [];
  let bestArea = -Infinity;
  for (const polygon of result) {
    if (!Array.isArray(polygon) || polygon.length === 0) continue;
    const outer = polygon[0];
    if (!outer || outer.length < 3) continue;
    const area = Math.abs(signedArea(outer));
    if (area > bestArea) {
      bestArea = area;
      best = outer.map((p: number[]) => [p[0], p[1]]);
    }
  }
  return best;
};
/**
 * Computes the union of two polygons.
 * @param poly1 The first polygon represented as an array of points.
 * @param poly2 The second polygon represented as an array of points.
 * @returns The resulting polygon after the union operation.
 */
const polygonUnion = (poly1: [number, number][], poly2: [number, number][]): [number, number][] => {
  if (!poly1 || poly1.length < 3) return poly2 ? poly2.slice() : [];
  if (!poly2 || poly2.length < 3) return poly1 ? poly1.slice() : [];

  try {
    // polygon-clipping expects polygons as arrays of rings: polygon = [ ring1, ring2?, ... ]
    const p1 = [poly1.map(p => [p[0], p[1]])];
    const p2 = [poly2.map(p => [p[0], p[1]])];
    const result = (polygonClipping as any).union(p1, p2);
    return pickLargestOuterRing(result);
  } catch (err) {
    // On failure, return original poly1 as a safe fallback
    console.warn('polygonUnion failed', err);
    return poly1.slice();
  }
};
/**
 * Computes the difference between two polygons.
 * @param poly1 The first polygon represented as an array of points.
 * @param poly2 The second polygon represented as an array of points.
 * @returns The resulting polygon after subtracting poly2 from poly1.
 */
const polygonDifference = (poly1: [number, number][], poly2: [number, number][]): [number, number][] => {
  if (!poly1 || poly1.length < 3) return [];
  if (!poly2 || poly2.length < 3) return poly1.slice();

  try {
    const p1 = [poly1.map(p => [p[0], p[1]])];
    const p2 = [poly2.map(p => [p[0], p[1]])];
    const result = (polygonClipping as any).difference(p1, p2);
    return pickLargestOuterRing(result);
  } catch (err) {
    console.warn('polygonDifference failed', err);
    return poly1.slice();
  }
};
/**
 * Merges two selection areas based on the specified merge type.
 * @param mainAreaPoints The main selection area points.
 * @param tmpAreaPoints The temporary selection area points.
 * @param mergeType The type of merge operation: 'add', 'subtract', or 'none'.
 * @returns The merged selection area points.
 */
const mergePoints = (
  mainAreaPoints: [number, number][],
  tmpAreaPoints: [number, number][],
  mergeType: 'add' | 'subtract' | 'none',
): [number, number][] => {
  if (mergeType === 'none') return mainAreaPoints;

  if (mergeType === 'add') {
    // Use polygon union to merge shapes
    return polygonUnion(mainAreaPoints, tmpAreaPoints);
  } else if (mergeType === 'subtract') {
    // Use polygon difference to subtract the tmp area from the main one
    return polygonDifference(mainAreaPoints, tmpAreaPoints);
  }
  return mainAreaPoints;
};
/**
 * Draws a ellipse within a bounding box defined by top-left corner, width, and height.
 * @param topX The x-coordinate of the top-left corner.
 * @param topY The y-coordinate of the top-left corner.
 * @param widthX The width of the bounding box.
 * @param widthY The height of the bounding box.
 * @param numPoints The number of points to generate along the ellipse.
 * @returns An array of points representing the ellipse.
 */
const getEllipsisPoints = (topX: number, topY: number, widthX: number, widthY: number, numPoints: number) => {
  const points: [number, number][] = [];
  for (let i = 0; i < numPoints; i++) {
    const angle = (i / numPoints) * 2 * Math.PI;
    const x = topX + widthX / 2 + (widthX / 2) * Math.cos(angle);
    const y = topY + widthY / 2 + (widthY / 2) * Math.sin(angle);
    points.push([x, y]);
  }
  return points;
};
/**
 * Computes an inset polygon using ClipperLib.
 * @param poly The polygon points.
 * @param distance The inset distance.
 * @returns The inset polygon points.
 */
const computeInsetPolygon = (poly: [number, number][], distance: number): [number, number][] => {
  const CLIPPER_SCALE = 1000;
  if (!poly || poly.length < 3 || distance <= 0) return poly.slice();

  try {
    // Convert to Clipper integer path
    const path = poly.map(p => ({ X: Math.round(p[0] * CLIPPER_SCALE), Y: Math.round(p[1] * CLIPPER_SCALE) }));

    const co = new (ClipperLib as any).ClipperOffset(/* miterLimit */ 2, /* arcTolerance */ 0.25);
    // Use round joins to avoid spikes; end type closed polygon
    co.AddPath(path, (ClipperLib as any).JoinType.jtRound, (ClipperLib as any).EndType.etClosedPolygon);

    const solution = new (ClipperLib as any).Paths();
    // Clipper expects positive expansion, so negative distance yields an inset
    co.Execute(solution, -Math.round(distance * CLIPPER_SCALE));

    if (!solution || solution.length === 0) {
      // fallback to centroid-scale if Clipper produced nothing
      throw new Error('clipper produced empty result');
    }

    // Pick the largest resulting ring
    let best = solution[0];
    let bestArea = Math.abs(signedArea(best.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE])));
    for (const sol of solution) {
      const area = Math.abs(signedArea(sol.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE])));
      if (area > bestArea) {
        bestArea = area;
        best = sol;
      }
    }

    const res = best.map((pt: any) => [pt.X / CLIPPER_SCALE, pt.Y / CLIPPER_SCALE] as [number, number]);

    // Final safety: if inset touches the boundary or is degenerate, fallback to simple centroid scaling
    const outArea = Math.abs(signedArea(res));
    const origArea = Math.abs(signedArea(poly));
    if (!(outArea > 1e-6 && outArea < origArea)) throw new Error('clipper produced degenerate inset');

    return res;
  } catch (err) {
    // console.warn('Clipper offset failed, falling back', err);
    // Fallback: simple centroid scale (previous behavior)
    const n = poly.length;
    const centroid = poly.reduce((acc, p) => [acc[0] + p[0], acc[1] + p[1]], [0, 0] as [number, number]);
    centroid[0] /= n;
    centroid[1] /= n;
    return poly.map(p => {
      const dx = p[0] - centroid[0];
      const dy = p[1] - centroid[1];
      const len = Math.hypot(dx, dy);
      if (len === 0) return p;
      const scale = Math.max(0, len - distance) / len;
      return [centroid[0] + dx * scale, centroid[1] + dy * scale] as [number, number];
    });
  }
};

const drawFeatherLine = (ctx: CanvasRenderingContext2D, areaPoints: [number, number][], feather: number) => {
  // Draw a line that is inset within the selection area to represent feathering
  ctx.strokeStyle = 'rgba(255, 0, 255, 0.5)';
  ctx.fillStyle = 'rgba(0, 0, 0, 0.0)';
  ctx.lineDashOffset = 0;
  ctx.lineWidth = 1;
  ctx.setLineDash([8, 8]);

  const points = computeInsetPolygon(areaPoints, feather);

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
  // Point refs
  const startPoint = useRef<[number, number] | null>(null);
  const endPoint = useRef<[number, number] | null>(null);
  const currentPoint = useRef<[number, number] | null>(null);

  // Area points refs
  const areaPoints = useRef<[number, number][]>([]);
  const tmpAreaPoints = useRef<[number, number][]>([]);
  const mergeType = useRef<'add' | 'subtract' | 'none'>('none');

  const [shape, setShape] = useState<'rect' | 'circle' | 'lasso'>('rect');
  const [style, setStyle] = useState<'normal' | 'ratio' | 'pixels'>('normal');
  const [feather, setFeather] = useNumericInput(0, { min: 0, max: 1000, step: 1 });
  const { onKeyDown, onPaste } = useNumericInputValidation();

  useEffect(() => {
    const cursor = toSvgCursor(faPlus);
    projectCursor.next({ cursor, origin: [10, 10] });
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
    /**
     * Handle mouse events for selection tool.
     */
    const mouseDown = canvasMouse$.pipe(filter(e => e.type === 'down')).subscribe(({ x, y, event }) => {
      startPoint.current = [x, y];

      // Determine merge type based on modifier keys
      const merge = event.shiftKey ? 'add' : event.ctrlKey ? 'subtract' : 'none';
      mergeType.current = merge;

      if (merge === 'none') {
        areaPoints.current = [[x, y]];
      } else if (merge === 'add' || merge === 'subtract') {
        tmpAreaPoints.current = [[x, y]];
      } else areaPoints.current = [];
    });
    const mouseMove = canvasMouse$.pipe(filter(e => e.type === 'move')).subscribe(({ x, y, isMouseDown }) => {
      currentPoint.current = [x, y];

      // If we are drawing a new selection, add points to the main array
      if (isMouseDown && mergeType.current === 'none') {
        if (shape === 'lasso') areaPoints.current.push([x, y]);
        else if (shape === 'rect')
          areaPoints.current = [
            [startPoint.current?.[0] ?? 0, startPoint.current?.[1] ?? 0],
            [x, startPoint.current?.[1] ?? 0],
            [x, y],
            [startPoint.current?.[0] ?? 0, y],
          ];
        else if (shape === 'circle') {
          areaPoints.current = [];
          const sx = startPoint.current?.[0] ?? 0;
          const sy = startPoint.current?.[1] ?? 0;
          const topX = Math.min(sx, x);
          const topY = Math.min(sy, y);
          const w = Math.abs(x - sx);
          const h = Math.abs(y - sy);
          getEllipsisPoints(topX, topY, w, h, 64).forEach(p => areaPoints.current.push(p));
        }
      }
      // If we are modifying an existing selection, add points to the temp array
      else if (isMouseDown && (mergeType.current === 'add' || mergeType.current === 'subtract')) {
        if (shape === 'lasso') tmpAreaPoints.current.push([x, y]);
        else if (shape === 'rect')
          tmpAreaPoints.current = [
            [startPoint.current?.[0] ?? 0, startPoint.current?.[1] ?? 0],
            [x, startPoint.current?.[1] ?? 0],
            [x, y],
            [startPoint.current?.[0] ?? 0, y],
          ];
        else if (shape === 'circle') {
          tmpAreaPoints.current = [];
          const sx = startPoint.current?.[0] ?? 0;
          const sy = startPoint.current?.[1] ?? 0;
          const topX = Math.min(sx, x);
          const topY = Math.min(sy, y);
          const w = Math.abs(x - sx);
          const h = Math.abs(y - sy);
          getEllipsisPoints(topX, topY, w, h, 64).forEach(p => tmpAreaPoints.current.push(p));
        }
      }
    });
    const mouseUp = canvasMouse$.pipe(filter(e => e.type === 'up')).subscribe(({ x, y }) => {
      endPoint.current = [x, y];

      if (tmpAreaPoints.current.length > 2) {
        areaPoints.current = mergePoints(areaPoints.current, tmpAreaPoints.current, mergeType.current);
      }
      tmpAreaPoints.current = [];

      const projectId = projects.activeProjectId;
      if (!projectId) return;
      window.tools.selection.setArea(projectId, areaPoints.current);

      // Immediately redraw the merged area on mouse up so the user sees the result
      canvasGizmos.next({
        action: 'draw',
        callback: ctx => drawCall(ctx, areaPoints.current, tmpAreaPoints.current, feather.num),
      });
    });

    return () => {
      mouseDown.unsubscribe();
      mouseMove.unsubscribe();
      mouseUp.unsubscribe();
    };
  }, [shape, feather]);

  // Draw rectangle/circle on the gizmos canvas
  useEffect(() => {
    let animationFrameId: number;
    const animation = () => {
      canvasGizmos.next({
        action: 'draw',
        callback: ctx => drawCall(ctx, areaPoints.current, tmpAreaPoints.current, feather.num),
      });
      animationFrameId = requestAnimationFrame(animation);
    };
    animationFrameId = requestAnimationFrame(animation);
    return () => cancelAnimationFrame(animationFrameId);
  }, [feather.num]);

  return (
    <div className="flex w-full items-center gap-4">
      <div>
        <Select value={shape} onSelect={setShape}>
          <Option value="rect">
            <FontAwesomeIcon icon={faSquare} />
          </Option>
          <Option value="circle">
            <FontAwesomeIcon icon={faCircle} />
          </Option>
          <Option value="lasso">
            <FontAwesomeIcon icon={faLasso} />
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
          <Select value={style} onSelect={setStyle} className="w-40">
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
