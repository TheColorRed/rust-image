import { colorPickerRequest, colorPickerRequest$, finishColorPick } from '@/src/state/color-picker';
import { useObservable } from 'react-rx';
import { ColorPickerDialog } from './color-picker-dialog';

/**
 * Shows the color picker whenever something asks for it with `pickColor` (`src/state/color-picker.ts`), and answers the request. It is mounted
 * once, at the root of the app, so the editor, the plugin editor and a plugin's control all open the same picker.
 */
export function ColorPickerHost() {
  const request = useObservable(colorPickerRequest$, colorPickerRequest.value);
  // A new request is a new picker, so its colors start from the color it was asked with and not the one before.
  return request ? <ColorPickerDialog key={request.id} request={request} onDone={finishColorPick} /> : null;
}
