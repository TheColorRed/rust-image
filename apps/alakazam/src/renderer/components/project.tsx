import { AppContext } from '@/app';
import { canvasGizmos$, canvasMouse, projectCursor$ } from '@/events/project';
import { useContext, useEffect, useRef, useState } from 'react';
import { filter, fromEvent } from 'rxjs';

export function Project() {
  const [renderData, setRenderData] = useState<ImageBitmap | null>(null);
  const [zoom, setZoom] = useState(1);
  const [isMouseDown, setIsMouseDown] = useState(false);

  const divRef = useRef<HTMLDivElement>(null);
  const { projects } = useContext(AppContext);

  const canvas = useRef<HTMLCanvasElement>(null);
  const canvasGizmos = useRef<HTMLCanvasElement>(null);
  const renderedDataRef = useRef<ImageBitmap | null>(null);

  // Listen to project composite changes and cursor changes
  useEffect(() => {
    const unsubscribe = window.alakazam.projects.onCompositeChanged(composite => {
      console.log('Composite changed, updating render data');
      let imageData = new ImageData(new Uint8ClampedArray(composite.data), composite.width, composite.height);
      createImageBitmap(imageData).then(bitmap => setRenderData(bitmap));
    });

    const cursorSubscription = projectCursor$.subscribe(({ cursor, origin }) => {
      if (!divRef.current) return;
      const cursorOrigin = origin ? `${origin[0]} ${origin[1]}, auto` : 'auto';
      divRef.current.style.cursor = `url('${cursor}') ${cursorOrigin}`;
    });

    return () => {
      unsubscribe();
      cursorSubscription.unsubscribe();
    };
  }, []);

  // Handle mouse events on the canvas
  useEffect(() => {
    const calculateMousePosition = (e: MouseEvent) => {
      if (!canvas.current) return { x: 0, y: 0 };
      const rect = canvas.current.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      return { x, y };
    };

    const mouseDownSubscription = fromEvent(canvas.current!, 'mousedown').subscribe((e: Event) => {
      setIsMouseDown(true);
      const mouseEvent = e as MouseEvent;
      const { x, y } = calculateMousePosition(mouseEvent);
      canvasMouse.next({ x, y, type: 'down', isMouseDown: true, event: mouseEvent });
    });

    const mouseMoveSubscription = fromEvent(canvas.current!, 'mousemove').subscribe((e: Event) => {
      const mouseEvent = e as MouseEvent;
      const { x, y } = calculateMousePosition(mouseEvent);
      canvasMouse.next({ x, y, type: 'move', isMouseDown, event: mouseEvent });
    });

    const mouseUpSubscription = fromEvent(canvas.current!, 'mouseup').subscribe((e: Event) => {
      setIsMouseDown(false);
      const mouseEvent = e as MouseEvent;
      const { x, y } = calculateMousePosition(mouseEvent);
      canvasMouse.next({ x, y, type: 'up', isMouseDown: false, event: mouseEvent });
    });

    return () => {
      mouseDownSubscription.unsubscribe();
      mouseMoveSubscription.unsubscribe();
      mouseUpSubscription.unsubscribe();
    };
  }, [isMouseDown]);

  // Load initial composite image
  useEffect(() => {
    if (!projects.activeProjectId) return;
    window.alakazam.projects.getComposite(projects.activeProjectId).then(composite => {
      let imageData = new ImageData(new Uint8ClampedArray(composite.data), composite.width, composite.height);
      createImageBitmap(imageData).then(bitmap => setRenderData(bitmap));
    });
  }, [projects.activeProjectId]);

  // Render the image to the canvas whenever renderData changes
  useEffect(() => {
    if (!canvas.current || !(renderData instanceof ImageBitmap)) return;

    const canvasRef = canvas.current;
    try {
      // Avoid redrawing the same ImageBitmap if it's already been rendered
      if (renderedDataRef.current === renderData) return;
      const ctx = canvasRef.getContext('2d');
      if (!ctx) throw new Error('Failed to get canvas context');

      // size the main canvas to the image
      canvasRef.width = renderData.width;
      canvasRef.height = renderData.height;

      // also size the gizmos overlay to match so drawings align
      if (canvasGizmos.current) {
        canvasGizmos.current.width = renderData.width;
        canvasGizmos.current.height = renderData.height;
        const gCtx = canvasGizmos.current.getContext('2d');
        if (gCtx) gCtx.clearRect(0, 0, canvasGizmos.current.width, canvasGizmos.current.height);
      }

      ctx.drawImage(renderData, 0, 0);
      renderedDataRef.current = renderData;
    } catch (error) {
      console.error('Error loading image:', error);
    }
  }, [renderData]);

  // Listen for gizmo draw events
  useEffect(() => {
    const gizmosClearSubscription = canvasGizmos$.pipe(filter(e => e.action === 'clear')).subscribe(() => {
      if (!canvasGizmos.current) return;
      const ctx = canvasGizmos.current.getContext('2d');
      if (!ctx) return;
      ctx.clearRect(0, 0, canvasGizmos.current.width, canvasGizmos.current.height);
    });

    const gizmosDrawSubscription = canvasGizmos$.pipe(filter(e => e.action === 'draw')).subscribe(event => {
      if (!canvasGizmos.current) return;
      const ctx = canvasGizmos.current.getContext('2d');
      if (!ctx) return;
      event.callback?.(ctx);
    });

    return () => {
      gizmosClearSubscription.unsubscribe();
      gizmosDrawSubscription.unsubscribe();
    };
  }, []);

  // const project = useMemo(() => getImage(projects.activeProjectId), [projects.activeProjectId]);

  // useLayoutEffect(() => {
  //   if (!divRef.current) return;
  //   if (!project || !project.firstLoad || !projects.activeProjectId) return;
  //   const div = divRef.current;
  //   const { width, height } = project.imageData ?? { width: 0, height: 0 };
  //   // Set the zoom to fit the image to the screen
  //   const rect = div.getBoundingClientRect();
  //   const zoomX = (rect.width - 100) / width;
  //   const zoomY = (rect.height - 100) / height;
  //   const zoom = Math.min(zoomX, zoomY, 1);

  //   updateImage(projects.activeProjectId, {
  //     ...project,
  //     zoom,
  //     firstLoad: false,
  //   });
  // }, [project]);

  // if (!project) return null;

  return (
    <div ref={divRef} className="bg-medium relative flex h-full w-full items-center justify-center overflow-hidden">
      <canvas
        className="pointer-events-none absolute z-10"
        ref={canvasGizmos}
        style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }}
      />
      <canvas ref={canvas} style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }} />
    </div>
  );
}
