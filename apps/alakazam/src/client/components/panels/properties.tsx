import { useNumericInputValidation } from '@/client/hooks/numeric-input-validation';
import { titleCase } from '@/client/lib/strings';
import { useNumericInput } from '@/client/lib/util';
import { Button } from '@/client/ui/button';
import { Input } from '@/client/ui/input';
import { Tooltip } from '@/client/ui/tooltip';
import { Anchor } from '@/server/preload';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faArrowDown,
  faArrowDownLeft,
  faArrowDownRight,
  faArrowLeft,
  faArrowRight,
  faArrowUp,
  faArrowUpLeft,
  faArrowUpRight,
  faFile,
  faImage,
  IconDefinition,
} from '@fortawesome/sharp-light-svg-icons';
import { faSquare } from '@fortawesome/sharp-solid-svg-icons';
import { LayerMetadata } from 'bindings';
import { createContext, useCallback, useContext, useEffect, useState } from 'react';

const PropertiesContext = createContext({
  activeLayer: null as LayerMetadata | null,
});

function AdjustmentLayerProperties() {
  const { activeLayer } = useContext(PropertiesContext);

  return <h3 className="text-base font-medium">{titleCase(activeLayer?.adjustmentType ?? '')}</h3>;
}

function AnchorButton({ tooltip, onClick, icon }: { tooltip: string; onClick: () => void; icon: IconDefinition }) {
  return (
    <Tooltip content={tooltip} position="above">
      <Button variant="icon" aspect="square" size="md" onClick={onClick}>
        <FontAwesomeIcon icon={icon} />
      </Button>
    </Tooltip>
  );
}

function DocumentProperties() {
  const [width, setWidth] = useNumericInput(800, { min: 1 });
  const [height, setHeight] = useNumericInput(600, { min: 1 });

  return (
    <div className="space-y-4">
      <h3 className="text-base font-medium">
        <FontAwesomeIcon icon={faFile} className="mr-2" size="lg" />
        Document
      </h3>
      <div>
        <div className="grid grid-cols-3 items-center gap-2">
          <div>Size</div>
          <Input suffix="px" selectFocus value={width.input} onChange={e => setWidth(Number(e))} />
          <Input suffix="px" selectFocus value={height.input} onChange={e => setHeight(Number(e))} />
        </div>
      </div>
    </div>
  );
}

function LayerProperties() {
  const { activeLayer } = useContext(PropertiesContext);
  const [width, setWidth] = useNumericInput(0, { min: 0 });
  const [height, setHeight] = useNumericInput(0, { min: 0 });
  const [x, setX] = useNumericInput(0);
  const [y, setY] = useNumericInput(0);
  const [angle, setAngle] = useNumericInput(0, { min: 0, max: 360 });
  const { onKeyDown, onPaste } = useNumericInputValidation();

  useEffect(() => {
    if (activeLayer) {
      setWidth(activeLayer.width);
      setHeight(activeLayer.height);
      setX(activeLayer.x);
      setY(activeLayer.y);
      setAngle(activeLayer.angle);
    }
  }, [activeLayer]);

  const updateProperty = useCallback(
    (property: 'width' | 'height' | 'x' | 'y' | 'angle', value: number, localOnly = false) => {
      if (!activeLayer) return;

      if (!localOnly) {
        if (property === 'width' || property === 'height') {
          let [newWidth, newHeight] = [undefined as number | undefined, undefined as number | undefined];
          if (property === 'width') newWidth = value;
          if (property === 'height') newHeight = value;
          window.alakazam.transform.resizeLayer(activeLayer.projectId, activeLayer.id, {
            width: newWidth,
            height: newHeight,
          });
        } else if (property === 'x' || property === 'y') {
          let [newX, newY] = [undefined as number | undefined, undefined as number | undefined];
          if (property === 'x') newX = value;
          if (property === 'y') newY = value;
          window.alakazam.transform.positionLayer(activeLayer.projectId, activeLayer.id, {
            x: newX,
            y: newY,
          });
        } else if (property === 'angle') {
          window.alakazam.transform.rotateLayer(activeLayer.projectId, activeLayer.id, value);
        }
      }

      if (property === 'width') setWidth(value);
      else if (property === 'height') setHeight(value);
      else if (property === 'x') setX(value);
      else if (property === 'y') setY(value);
      else if (property === 'angle') setAngle(value);
    },
    [activeLayer],
  );

  const setAnchor = useCallback(
    (anchor: Anchor) => {
      if (!activeLayer) return;
      window.alakazam.transform.anchorLayer(activeLayer.projectId, activeLayer.id, anchor);
    },
    [activeLayer],
  );

  return (
    <div className="space-y-2">
      <h3 className="text-base font-medium">
        <FontAwesomeIcon icon={faImage} className="mr-2" size="lg" />
        Raster
      </h3>
      <div className="grid grid-cols-3 items-center gap-2">
        <div>Size</div>
        <Input
          suffix="px"
          selectFocus
          value={width.input}
          onChange={e => updateProperty('width', Number(e), true)}
          onBlur={e => updateProperty('width', Number(e))}
          onPaste={onPaste}
          onKeyDown={onKeyDown}
        />
        <Input
          suffix="px"
          selectFocus
          value={height.input}
          onChange={e => updateProperty('height', Number(e), true)}
          onBlur={e => updateProperty('height', Number(e))}
          onPaste={onPaste}
          onKeyDown={onKeyDown}
        />
      </div>
      <div className="grid grid-cols-3 items-center gap-2">
        <div>Position</div>
        <Input
          suffix="px"
          selectFocus
          value={x.input}
          onChange={e => updateProperty('x', Number(e))}
          onPaste={onPaste}
          onKeyDown={onKeyDown}
        />
        <Input
          suffix="px"
          selectFocus
          value={y.input}
          onChange={e => updateProperty('y', Number(e))}
          onPaste={onPaste}
          onKeyDown={onKeyDown}
        />
      </div>
      <div className="grid grid-cols-3 items-center gap-2">
        <div>Rotation</div>
        <Input
          suffix="°"
          className="col-span-2"
          selectFocus
          value={angle.input}
          onChange={e => updateProperty('angle', Number(e), true)}
          onBlur={e => updateProperty('angle', Number(e))}
          onPaste={onPaste}
          onKeyDown={e => {
            onKeyDown(e);
            if (e.key === 'Enter') {
              updateProperty('angle', Number((e.target as HTMLInputElement).value));
            } else if (e.key === 'Escape') {
              if (activeLayer) setAngle(0);
            }
          }}
        />
      </div>
      <div>
        <div className="grid grid-cols-3 items-center">
          <div>Anchor</div>
          <div />
          <div className="col-span-1 grid grid-cols-3 gap-1">
            <AnchorButton tooltip="Top Left" icon={faArrowUpLeft} onClick={() => setAnchor('top-left')} />
            <AnchorButton tooltip="Top Center" icon={faArrowUp} onClick={() => setAnchor('top-center')} />
            <AnchorButton tooltip="Top Right" icon={faArrowUpRight} onClick={() => setAnchor('top-right')} />
            <AnchorButton tooltip="Middle Left" icon={faArrowLeft} onClick={() => setAnchor('center-left')} />
            <AnchorButton tooltip="Center" icon={faSquare} onClick={() => setAnchor('center')} />
            <AnchorButton tooltip="Middle Right" icon={faArrowRight} onClick={() => setAnchor('center-right')} />
            <AnchorButton tooltip="Bottom Left" icon={faArrowDownLeft} onClick={() => setAnchor('bottom-left')} />
            <AnchorButton tooltip="Bottom Center" icon={faArrowDown} onClick={() => setAnchor('bottom-center')} />
            <AnchorButton tooltip="Bottom Right" icon={faArrowDownRight} onClick={() => setAnchor('bottom-right')} />
          </div>
        </div>
      </div>
    </div>
  );
}

export function PropertiesPanel() {
  const [activeLayer, setActiveLayer] = useState<LayerMetadata | null>(null);

  useEffect(() => {
    const unsubscribe = window.alakazam.projects.onProjectChanged(project => {
      const layer = project.activeLayers.length > 0 ? project.activeLayers[0] : null;
      setActiveLayer(layer);
    });
    return () => {
      unsubscribe();
    };
  }, []);

  return (
    <PropertiesContext.Provider value={{ activeLayer }}>
      <div className="min-h-[200px] space-y-4">
        <h2 className="text-lg font-medium">Properties</h2>
        {activeLayer && activeLayer.adjustmentType ? (
          <AdjustmentLayerProperties />
        ) : activeLayer ? (
          <LayerProperties />
        ) : (
          <DocumentProperties />
        )}
      </div>
    </PropertiesContext.Provider>
  );
}
PropertiesPanel.displayName = 'PropertiesPanel';
