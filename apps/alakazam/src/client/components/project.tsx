import { AppContext } from '@/client/app';
import { useContext, useEffect, useRef, useState } from 'react';

export function Project() {
  const [renderData, setRenderData] = useState<ImageBitmap | null>(null);
  const [zoom, setZoom] = useState(1);

  const divRef = useRef<HTMLDivElement>(null);
  const { projects } = useContext(AppContext);

  const canvas = useRef<HTMLCanvasElement>(null);
  const renderedDataRef = useRef<ImageBitmap | null>(null);

  useEffect(() => {
    const unsubscribe = window.alakazam.projects.onCompositeChanged(composite => {
      console.log('Composite changed, updating render data');
      let imageData = new ImageData(new Uint8ClampedArray(composite.data), composite.width, composite.height);
      createImageBitmap(imageData).then(bitmap => setRenderData(bitmap));
    });
    return () => {
      unsubscribe();
    };
  }, []);

  useEffect(() => {
    if (!projects.activeProjectId) return;
    window.alakazam.projects.getComposite(projects.activeProjectId).then(composite => {
      let imageData = new ImageData(new Uint8ClampedArray(composite.data), composite.width, composite.height);
      createImageBitmap(imageData).then(bitmap => setRenderData(bitmap));
    });
  }, [projects.activeProjectId]);

  useEffect(() => {
    if (!canvas.current || !(renderData instanceof ImageBitmap)) return;

    const canvasRef = canvas.current;
    try {
      // Avoid redrawing the same ImageBitmap if it's already been rendered
      if (renderedDataRef.current === renderData) return;
      const ctx = canvasRef.getContext('2d');
      if (!ctx) throw new Error('Failed to get canvas context');

      canvasRef.width = renderData.width;
      canvasRef.height = renderData.height;

      ctx.drawImage(renderData, 0, 0);
      renderedDataRef.current = renderData;
    } catch (error) {
      console.error('Error loading image:', error);
    }
  }, [renderData]);

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
    <div ref={divRef} className="bg-medium flex h-full w-full items-center justify-center overflow-hidden">
      <canvas ref={canvas} style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }} />
    </div>
  );
}
