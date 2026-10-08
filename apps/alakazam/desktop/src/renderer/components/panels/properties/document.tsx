import { useNumericInput } from '@/lib/util';
import { Input } from '@/ui/input';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFile } from '@fortawesome/sharp-light-svg-icons';

export function DocumentProperties() {
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
