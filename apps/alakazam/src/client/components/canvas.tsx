import { AppContext } from '@/client/app';
import { useContext, useEffect, useRef, useState } from 'react';

export interface ImageProps {
  zoom?: number;
  filePath?: string;
  /**
   * Renders the image from the project at the specified index.
   */
  imageId?: string | null;
}

export function ProjectCanvas({ zoom = 1, filePath, imageId }: ImageProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [renderData, setRenderData] = useState<ImageData | null>(null);
  const renderedDataRef = useRef<ImageData | null>(null);
  const { projects } = useContext(AppContext);

  // useEffect(() => {
  //   if (!projects.activeProjectId) return;
  //   console.log('Fetching project data for', projects.activeProjectId);
  //   window.alakazam.projects.getComposite(projects.activeProjectId).then(project => {
  //     const composite = project.composite();
  //     setRenderData(composite);
  //   });
  // }, [projects.activeProjectId]);

  // useEffect(() => {
  //   (async function () {
  //     let data: ImageData | null = null;
  //     // if (typeof filePath === 'string') data = await window.alakazam.openProject(filePath);
  //     // else if (imageId !== undefined) {
  //     const projectId = appProjects.activeProjectId;
  //     // data = project?.imageData ?? null;
  //     // }
  //     setRenderData(data);
  //   })();
  // }, [appProjects.activeProjectId, filePath]);

  useEffect(() => {
    if (!canvas.current || !(renderData instanceof ImageData)) return;

    const canvasRef = canvas.current;
    try {
      // Avoid redrawing the same ImageData if it's already been rendered
      if (renderedDataRef.current === renderData) return;
      const ctx = canvasRef.getContext('2d');
      if (!ctx) throw new Error('Failed to get canvas context');

      canvasRef.width = renderData.width;
      canvasRef.height = renderData.height;

      ctx.putImageData(renderData, 0, 0);
      renderedDataRef.current = renderData;
    } catch (error) {
      console.error('Error loading image:', error);
    }
  }, [renderData]);

  return (
    <>
      <canvas ref={canvas} style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }} />
    </>
  );
}
