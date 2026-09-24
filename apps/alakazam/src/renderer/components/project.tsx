import { AppContext } from '@/app';
import { canvasGizmos$, canvasMouse, projectCursor$ } from '@/events/project';
import { useContext, useEffect, useRef, useState } from 'react';
import { filter, fromEvent } from 'rxjs';

export function Project() {
  const [renderData, setRenderData] = useState<ImageBitmap | null>(null);
  const [transparentRenderData, setTransparentRenderData] = useState<ImageBitmap | null>(null);
  const [zoom, setZoom] = useState(1);
  // const [isLargeCursor, setIsLargeCursor] = useState(false);
  const [isRendering, setIsRendering] = useState(false);
  const [isMouseDown, setIsMouseDown] = useState(false);

  const divRef = useRef<HTMLDivElement>(null);
  const largeCursorRef = useRef<HTMLDivElement>(null);
  const { projects } = useContext(AppContext);

  const canvas = useRef<HTMLCanvasElement>(null);
  const canvasGizmosRef = useRef<HTMLCanvasElement>(null);
  const renderedDataRef = useRef<ImageBitmap>(null);
  const emptyCanvasRef = useRef<HTMLCanvasElement>(null);

  // Listen to project composite changes and cursor changes
  useEffect(() => {
    const unsubscribe = window.alakazam.projects.onCompositeChanged(composite => {
      // console.log('Composite changed, updating render data');
      let imageData = new ImageData(new Uint8ClampedArray(composite.data), composite.width, composite.height);
      createImageBitmap(imageData).then(bitmap => setRenderData(bitmap));
    });

    window.alakazam.projects.getActiveProjectMetadata().then(async metadata => {
      const tmpImageData = await window.alakazam.layers.getTransparentImage(metadata!.width, metadata!.height);
      const imageData = new ImageData(
        new Uint8ClampedArray(tmpImageData.data),
        tmpImageData.width,
        tmpImageData.height,
      );

      createImageBitmap(imageData).then(bitmap => setTransparentRenderData(bitmap));
    });

    const cursorSubscription = projectCursor$.subscribe(({ cursor, isNativeCursor, origin, size }) => {
      if (!divRef.current) return;
      size = Array.isArray(size) ? Math.max(size[0], size[1]) : size;
      origin = origin ?? [0, 0];
      if (!largeCursorRef.current) return;
      largeCursorRef.current.style.width = `${size}px`;
      largeCursorRef.current.style.height = `${size}px`;
      if (size) {
        largeCursorRef.current.style.marginLeft = origin ? `-${origin[0]}px` : `-${size / 2}px`;
        largeCursorRef.current.style.marginTop = origin ? `-${origin[1]}px` : `-${size / 2}px`;
      } else {
        largeCursorRef.current.style.marginLeft = '';
        largeCursorRef.current.style.marginTop = '';
      }
      largeCursorRef.current.style.backgroundImage = isNativeCursor ? '' : `url('${cursor}')`;
      largeCursorRef.current.style.backgroundSize = 'contain';
      largeCursorRef.current.style.backgroundRepeat = 'no-repeat';
      divRef.current.style.cursor = isNativeCursor ? cursor : 'none';
    });

    return () => {
      unsubscribe();
      cursorSubscription.unsubscribe();
    };
  }, []);

  useEffect(() => {
    if (!isRendering) {
      largeCursorRef.current?.style.setProperty('display', 'none');
      return;
    }
    const handleMouseMove = (e: MouseEvent) => {
      if (!largeCursorRef.current) return;
      largeCursorRef.current.style.setProperty('display', 'block');
      largeCursorRef.current.style.left = `${e.clientX}px`;
      largeCursorRef.current.style.top = `${e.clientY}px`;
    };

    window.addEventListener('mousemove', handleMouseMove);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
    };
  }, [isRendering]);

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

  // Handle large cursor rendering
  // Only render large cursor when in project area
  useEffect(() => {
    if (!divRef.current) return;
    const div = divRef.current;

    const handleMouseEnter = () => setIsRendering(true);
    const handleMouseLeave = () => setIsRendering(false);
    div.addEventListener('mouseenter', handleMouseEnter);
    div.addEventListener('mouseleave', handleMouseLeave);
    return () => {
      div.removeEventListener('mouseenter', handleMouseEnter);
      div.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, []);

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
      if (canvasGizmosRef.current) {
        canvasGizmosRef.current.width = renderData.width;
        canvasGizmosRef.current.height = renderData.height;
        const gCtx = canvasGizmosRef.current.getContext('2d');
        if (gCtx) gCtx.clearRect(0, 0, canvasGizmosRef.current.width, canvasGizmosRef.current.height);
      }

      ctx.drawImage(renderData, 0, 0);
      renderedDataRef.current = renderData;
    } catch (error) {
      console.error('Error loading image:', error);
    }
  }, [renderData]);

  useEffect(() => {
    if (!emptyCanvasRef.current || !(transparentRenderData instanceof ImageBitmap)) return;

    const canvasRef = emptyCanvasRef.current;
    try {
      const ctx = canvasRef.getContext('2d');
      if (!ctx) throw new Error('Failed to get canvas context');

      // size the empty canvas to the image
      canvasRef.width = transparentRenderData.width;
      canvasRef.height = transparentRenderData.height;

      ctx.drawImage(transparentRenderData, 0, 0);
    } catch (error) {
      console.error('Error loading transparent image:', error);
    }
  }, [transparentRenderData]);

  // Listen for gizmo draw events
  useEffect(() => {
    const gizmosClearSubscription = canvasGizmos$.pipe(filter(e => e.action === 'clear')).subscribe(() => {
      if (!canvasGizmosRef.current) return;
      const ctx = canvasGizmosRef.current.getContext('2d');
      if (!ctx) return;
      ctx.clearRect(0, 0, canvasGizmosRef.current.width, canvasGizmosRef.current.height);
    });

    const gizmosDrawSubscription = canvasGizmos$.pipe(filter(e => e.action === 'draw')).subscribe(event => {
      if (!canvasGizmosRef.current) return;
      const ctx = canvasGizmosRef.current.getContext('2d');
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
    <>
      <div ref={largeCursorRef} className="pointer-events-none absolute z-20" />
      <div ref={divRef} className="bg-medium relative flex h-full w-full items-center justify-center overflow-hidden">
        <canvas
          className="pointer-events-none absolute z-2"
          ref={canvasGizmosRef}
          style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }}
          data-project-gizmos
        />
        <canvas
          ref={canvas}
          style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }}
          className="absolute z-1"
          data-project-canvas
        />
        <canvas
          ref={emptyCanvasRef}
          style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }}
          className="pointer-events-none absolute"
          data-empty-canvas
        />
      </div>
    </>
  );
}
