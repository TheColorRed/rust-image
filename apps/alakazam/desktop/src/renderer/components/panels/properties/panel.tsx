import { PropertiesContext } from '@/components/panels/properties/properties';
import { useNumericInputValidation } from '@/hooks/numeric-input-validation';
import { useNumericInput } from '@/lib/util';
import { Button } from '@/ui/button';
import { Input } from '@/ui/input';
import { Tooltip } from '@/ui/tooltip';
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
  faImage,
  faSquare,
  IconDefinition,
} from '@fortawesome/sharp-light-svg-icons';
import { Anchor } from '@server/preload/alakazam';
import { useCallback, useContext, useEffect } from 'react';

function AnchorButton({
  tooltip,
  onClick,
  icon,
  value,
}: {
  tooltip: string;
  onClick: (value: Anchor) => void;
  icon: IconDefinition;
  value: Anchor;
}) {
  const { activeLayer } = useContext(PropertiesContext);

  return (
    <Tooltip content={tooltip} position="above">
      <Button
        variant="icon"
        aspect="square"
        size="md"
        onClick={() => onClick(value)}
        active={activeLayer?.anchor === value}
      >
        <FontAwesomeIcon icon={icon} />
      </Button>
    </Tooltip>
  );
}

export function RasterProperties() {
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
            <AnchorButton tooltip="Top Left" value="top-left" icon={faArrowUpLeft} onClick={setAnchor} />
            <AnchorButton tooltip="Top Center" value="top-center" icon={faArrowUp} onClick={setAnchor} />
            <AnchorButton tooltip="Top Right" value="top-right" icon={faArrowUpRight} onClick={setAnchor} />
            <AnchorButton tooltip="Middle Left" value="center-left" icon={faArrowLeft} onClick={setAnchor} />
            <AnchorButton tooltip="Center" value="center" icon={faSquare} onClick={setAnchor} />
            <AnchorButton tooltip="Middle Right" value="center-right" icon={faArrowRight} onClick={setAnchor} />
            <AnchorButton
              tooltip="Bottom Left"
              value="bottom-left"
              icon={faArrowDownLeft}
              onClick={v => setAnchor(v)}
            />
            <AnchorButton
              tooltip="Bottom Center"
              value="bottom-center"
              icon={faArrowDown}
              onClick={v => setAnchor(v)}
            />
            <AnchorButton
              tooltip="Bottom Right"
              value="bottom-right"
              icon={faArrowDownRight}
              onClick={v => setAnchor(v)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
