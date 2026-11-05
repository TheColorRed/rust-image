import { finishGradientPick, gradientPickerRequest, gradientPickerRequest$ } from '@/src/state/gradient-picker';
import { useObservable } from 'react-rx';
import { GradientPickerDialog } from './gradient-picker-dialog';

/**
 * Shows the gradient editor whenever something asks for it with `pickGradient` (`src/state/gradient-picker.ts`), and answers the request. It is
 * mounted once, at the root of the app, before the color picker host so a color picked for a stop is shown over the editor.
 */
export function GradientPickerHost() {
  const request = useObservable(gradientPickerRequest$, gradientPickerRequest.value);
  // A new request is a new editor, so it starts from the gradient it was asked with and not the one before.
  return request ? <GradientPickerDialog key={request.id} request={request} onDone={finishGradientPick} /> : null;
}
