import { AppContext } from '@/client/app';
import { useNumericInputValidation } from '@/client/hooks/numeric-input-validation';
import { cn, useNumericInput } from '@/client/lib/util';
import { Button, ButtonOptions } from '@/client/ui/button';
import { DragItem, DragListContainer } from '@/client/ui/drag';
import { Input } from '@/client/ui/input';
import { Option } from '@/client/ui/option';
import { Select } from '@/client/ui/select';
import { Tooltip } from '@/client/ui/tooltip';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faSquarePlus, faTrash } from '@fortawesome/sharp-light-svg-icons';
import { faCircleHalfStroke, faEye, faEyeSlash } from '@fortawesome/sharp-solid-svg-icons';
import { LayerMetadata, ProjectMetadata } from 'bindings';
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';

type NewLayerAction = 'layer-file' | 'layer-empty' | 'layer-adjustment' | 'group';

const LayersContext = createContext({
  activeProject: null as ProjectMetadata | null,
  projectMetadata: null as ProjectMetadata | null,
});

const CANVAS_THUMBNAIL_SIZE = 38;

function LayersHeader() {
  const [blendMode, setBlendMode] = useState('normal');
  const [opacity, setOpacity] = useNumericInput(100, { min: 0, max: 100, step: 1 });
  const { activeProject: project, projectMetadata } = useContext(LayersContext);
  const [activeLayers, setActiveLayers] = useState<string[]>([]);
  const { onKeyDown, onPaste } = useNumericInputValidation();

  useEffect(() => {
    if (!projectMetadata) return;
    setActiveLayers(projectMetadata.activeLayers.map(layer => layer.id));
  }, [projectMetadata]);

  useEffect(() => {
    if (!projectMetadata) return;
    const blendModes = projectMetadata.activeLayers[0]?.blendMode ?? 'normal';
    setBlendMode(blendModes);
    const opacities = (projectMetadata.activeLayers[0]?.opacity ?? 1) * 100;
    setOpacity(opacities);
  }, [projectMetadata]);

  useEffect(() => {
    if (!project || !projectMetadata) return;
    window.alakazam.projects.setBlendMode(project.id, activeLayers, blendMode);
  }, [project, blendMode]);

  useEffect(() => {
    if (!project || !projectMetadata) return;
    window.alakazam.projects.setOpacity(project.id, activeLayers, opacity.num / 100);
  }, [project, opacity.num]);

  return (
    <div className="flex flex-col justify-between border-b border-gray-700">
      <h2 className="text-lg font-medium">Layers</h2>
      <Select value={blendMode} onSelect={setBlendMode} disabled={activeLayers.length === 0}>
        <Option type="separator" title="Basic Blends" />
        <Option value="normal">Normal</Option>
        <Option value="average">Average</Option>
        <Option type="separator" title="Darken Blends" />
        <Option value="darken">Darken</Option>
        <Option value="darker-color">Darker Color</Option>
        <Option value="multiply">Multiply</Option>
        <Option value="color-burn">Color Burn</Option>
        <Option value="linear-burn">Linear Burn</Option>
        <Option type="separator" title="Lighten Blends" />
        <Option value="lighten">Lighten</Option>
        <Option value="lighter-color">Lighter Color</Option>
        <Option value="screen">Screen</Option>
        <Option value="color-dodge">Color Dodge</Option>
        <Option value="linear-dodge">Linear Dodge</Option>
        <Option type="separator" title="Overlay Blends" />
        <Option value="overlay">Overlay</Option>
        <Option value="soft-light">Soft Light</Option>
        <Option value="hard-light">Hard Light</Option>
        <Option value="vivid-light">Vivid Light</Option>
        <Option value="linear-light">Linear Light</Option>
        <Option value="pin-light">Pin Light</Option>
        <Option value="hard-mix">Hard Mix</Option>
        <Option type="separator" title="Difference Blends" />
        <Option value="difference">Difference</Option>
        <Option value="exclusion">Exclusion</Option>
        <Option value="subtract">Subtract</Option>
        <Option value="divide">Divide</Option>
        <Option type="separator" title="Color Blends" />
        <Option value="hue">Hue</Option>
        <Option value="saturation">Saturation</Option>
        <Option value="color">Color</Option>
        <Option value="luminosity">Luminosity</Option>
        <Option type="separator" title="Special Blends" />
        <Option value="glow">Glow</Option>
        <Option value="reflect">Reflect</Option>
        <Option value="phoenix">Phoenix</Option>
        <Option value="negation">Negation</Option>
        <Option value="grain-extract">Grain Extract</Option>
        <Option value="grain-merge">Grain Merge</Option>
      </Select>
      <Input
        className="mt-2 w-full"
        selectFocus
        disabled={activeLayers.length === 0}
        value={opacity.input}
        onChange={setOpacity}
        onKeyDown={onKeyDown}
        onPaste={onPaste}
        suffix="%"
      />
    </div>
  );
}

function LayerRow({ layer, setDraggable }: { layer: LayerMetadata; setDraggable: (draggable: boolean) => void }) {
  const { projectMetadata: project } = useContext(LayersContext);
  const Projects = window.alakazam.projects;
  const [visible, setVisible] = useState<boolean>(!!layer.visible);
  const [editingName, setEditingName] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [canvasData, setCanvasData] = useState<ImageData | null>(null);

  useEffect(() => {
    setVisible(!!layer.visible);
  }, [layer.visible]);

  const getLayerThumbnail = async (metadata: LayerMetadata) => {
    if (metadata.id !== layer.id || !project) return;
    const layerId = metadata.id;
    window.alakazam.layers.getLayerComposite(project.id, layerId, CANVAS_THUMBNAIL_SIZE).then(imageData => {
      if (!imageData) return setCanvasData(null);
      setCanvasData(new ImageData(new Uint8ClampedArray(imageData?.data ?? []), imageData.width, imageData.height));
    });
  };

  const selectLayer = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (!project) return;
      Projects.setActiveLayers(project.id, [layer.id]);
    },
    [project],
  );

  const toggleVisibility = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (!project) return;
      const newVisible = !visible;
      const layerId = layer.id;
      // Optimistically update the icon immediately
      setVisible(newVisible);
      Projects.setVisibility(project.id, [layerId], newVisible);
    },
    [project, layer.id, visible],
  );

  const onDoubleClickText = useCallback(() => {
    setEditingName(true);
    setDraggable(false);
  }, []);

  useEffect(() => {
    getLayerThumbnail(layer);
  }, [layer]);

  useEffect(() => {
    const releaseLayerCompositeChanged = window.alakazam.projects.onLayerCompositeChanged(layerMetadata => {
      getLayerThumbnail(layerMetadata);
    });
    return () => {
      releaseLayerCompositeChanged();
    };
  }, []);

  useEffect(() => {
    if (!canvasRef.current || !canvasData) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    try {
      // set canvas size and draw image data
      canvas.width = canvasData.width ?? 0;
      canvas.height = canvasData.height ?? 0;
      ctx.putImageData(canvasData, 0, 0);
    } catch (err) {
      console.error('Error drawing layer thumbnail', err);
    }
  }, [canvasData]);

  return (
    <div
      key={layer.id}
      data-layer-id={layer.id}
      className={cn('border-dark bg-default flex h-16 cursor-pointer items-center gap-2 border-b p-1.5', {
        'bg-blue-500/20': project?.activeLayers.some(activeLayer => activeLayer.id === layer.id),
      })}
      onClick={selectLayer}
    >
      <Button variant="icon" aspect="square" size="sm" onClick={toggleVisibility}>
        {visible ? <FontAwesomeIcon icon={faEye} /> : <FontAwesomeIcon icon={faEyeSlash} />}
      </Button>
      {canvasData && <canvas ref={canvasRef} className="border-dark max-h-16 max-w-16 border-2" />}
      {!canvasData && <div className="h-16 w-16" />}
      {!editingName && <div onDoubleClick={onDoubleClickText}>{layer.name}</div>}
      {editingName && (
        <div>
          <Input
            autoFocus
            defaultValue={layer.name}
            onBlur={e => {
              const newName = e.trim();
              if (newName && project) {
                Projects.renameLayer(project.id, layer.id, newName);
              }
              setEditingName(false);
              setDraggable(true);
            }}
            onKeyDown={e => {
              if (e.key === 'Enter') {
                const newName = (e.target as HTMLInputElement).value.trim();
                if (newName && project) {
                  Projects.renameLayer(project.id, layer.id, newName);
                }
                setEditingName(false);
                setDraggable(true);
              } else if (e.key === 'Escape') {
                setEditingName(false);
                setDraggable(true);
              }
            }}
          />
        </div>
      )}
    </div>
  );
}

function Layers() {
  const { activeProject: project, projectMetadata } = useContext(LayersContext);
  // const [layers, setLayers] = useState<LayerMetadata[]>([]);
  const [tooltipDisabled, setTooltipDisabled] = useState(false);
  const [draggable, setDraggable] = useState(true);
  const Projects = window.alakazam.projects;

  const onAddLayer = useCallback(
    async (action: NewLayerAction, options?: any) => {
      if (!project) return;
      let layerId: string | undefined = undefined;
      if (action === 'layer-file') {
        const file = (await window.alakazam.openFileDialog())?.[0];
        if (file) {
          layerId = (await Projects.addLayer(project.id, { type: 'file', filePath: file, name: 'New Layer' })).id;
        }
      } else if (action === 'layer-adjustment') {
        if (!options?.adjustmentType) return;
        // For now, just add an empty adjustment layer
        const layer = await Projects.addLayer(project.id, {
          type: 'adjustment',
          adjustmentType: options.adjustmentType,
          name: options.adjustmentType.replace(/-/g, ' ').replace(/\b\w/g, (c: string) => c.toUpperCase()),
        });
        Projects.moveLayerToTop(project.id, layer.id);
      } else if (action === 'layer-empty') {
        layerId = (await Projects.addLayer(project.id, { type: 'empty', name: 'New Layer' })).id;
      } else if (action === 'group') {
      }

      // If we added a new layer, make it active and position it above existing active layers
      if (!layerId) return;
      const active = await Projects.getActiveLayers(project.id);
      if (active.length > 0) {
        const firstActiveLayerOrderIdx = active[0].order;
        await Projects.moveLayerTo(project.id, layerId, firstActiveLayerOrderIdx + 1);
      }
      await Projects.setActiveLayers(project.id, [layerId]);
    },
    [project, project?.activeLayers],
  );

  const deleteLayers = useCallback(() => {
    if (!project) return;
    Projects.getActiveLayers(project.id).then(activeLayers => {
      const ids = activeLayers.map(layer => layer.id);
      window.alakazam.projects.deleteLayers(project.id, ids);
    });
  }, [project]);

  const onReorderLayers = useCallback(
    (newOrder: LayerMetadata[]) => {
      if (!project) return;
      const newIds = newOrder.map(layer => layer.id);
      console.log('Reordering layers to new IDs:', newIds);
      Projects.reorderLayers(project.id, newIds);
    },
    [project],
  );

  const handleOutsideClick = () => {
    if (!project) return;
    Projects.setActiveLayers(project.id, []);
  };

  return (
    <div className="flex h-full flex-col">
      <LayersHeader />
      <hr className="my-2" />
      <div className="flex-1" onClick={handleOutsideClick}>
        <DragListContainer
          draggable={draggable}
          onOrderChange={onReorderLayers}
          order={projectMetadata?.layers ?? []}
          dragDelay={300}
          lockX
        >
          {projectMetadata?.layers.map(layer => (
            <DragItem key={layer.id} className="bg-default h-16 w-full">
              <LayerRow setDraggable={setDraggable} layer={layer} />
            </DragItem>
          ))}
        </DragListContainer>
      </div>
      <div className="flex h-10 justify-end">
        <Tooltip content="Add Adjustment Layer" disabled={tooltipDisabled}>
          <Button variant="icon" aspect="square" className="h-full" onOptionsOpenChange={setTooltipDisabled}>
            <FontAwesomeIcon icon={faCircleHalfStroke} size="xl" />
            <ButtonOptions
              activationStyle="whole"
              placement="above"
              onOptionSelected={value => onAddLayer('layer-adjustment', { adjustmentType: value })}
            >
              <Option value="brightness-contrast">Brightness/Contrast</Option>
              <Option value="exposure">Exposure</Option>
              <Option value="hue-saturation">Hue/Saturation</Option>
              <Option value="color-balance">Color Balance</Option>
              <Option value="black-white">Black & White</Option>
              <Option value="photo-filter">Photo Filter</Option>
              <Option value="channel-mixer">Channel Mixer</Option>
              <Option value="invert">Invert</Option>
              <Option value="posterize">Posterize</Option>
              <Option value="gradient-map">Gradient Map</Option>
              <Option value="solid-color">Solid Color</Option>
              <Option value="gradient">Gradient</Option>
              <Option value="pattern">Pattern</Option>
            </ButtonOptions>
          </Button>
        </Tooltip>
        <Tooltip content="Add Layer" disabled={tooltipDisabled}>
          <Button
            variant="icon"
            aspect="square"
            className="h-full"
            onClick={() => onAddLayer('layer-empty')}
            onOptionsOpenChange={setTooltipDisabled}
          >
            <FontAwesomeIcon icon={faSquarePlus} size="xl" />
            <ButtonOptions placement="above" onOptionSelected={onAddLayer}>
              <Option value="layer-file">New layer from file</Option>
              <Option value="layer-empty">New empty layer</Option>
              <Option value="group">New Group</Option>
            </ButtonOptions>
          </Button>
        </Tooltip>
        <Tooltip content="Delete Layer" className="whitespace-nowrap">
          <Button
            disabled={projectMetadata?.activeLayers.length === 0}
            variant="icon"
            aspect="square"
            className="h-full"
            onClick={() => deleteLayers()}
          >
            <FontAwesomeIcon icon={faTrash} size="xl" />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}

export function LayersPanel() {
  const { projects } = useContext(AppContext);
  const [activeProject, setActiveProject] = useState<ProjectMetadata | null>(null);
  const [projectMetadata, setProjectMetadata] = useState<ProjectMetadata | null>(null);

  useEffect(() => {
    if (!projects.activeProjectId) return;
    window.alakazam.projects.getProjectMetadata(projects.activeProjectId).then(setActiveProject);
    const releaseProjectChangeListener = window.alakazam.projects.onProjectChanged(metadata => {
      // console.log('Received project changed event in LayersPanel:', metadata);
      setProjectMetadata(metadata);
    });
    return () => {
      releaseProjectChangeListener();
    };
  }, [projects.activeProjectId]);

  useEffect(() => {
    if (!activeProject) return setProjectMetadata(null);
    window.alakazam.projects.getProjectMetadata(activeProject.id).then(d => {
      console.log('Fetched project metadata:', d);
      setProjectMetadata(d);
    });
  }, [activeProject]);

  return (
    <LayersContext.Provider value={{ activeProject, projectMetadata }}>
      <div className="h-full min-h-[200px] flex-1">
        <Layers />
      </div>
    </LayersContext.Provider>
  );
}
