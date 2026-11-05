import { curvePickerRequest, curvePickerRequest$, finishCurvePick } from '@/src/state/curve-picker';
import { useObservable } from 'react-rx';
import { CurvePickerDialog } from './curve-picker-dialog';

/** Shows the curve editor whenever something asks for it with `pickCurve` (`src/state/curve-picker.ts`), and answers the request. It is mounted once, at the root of the app. */
export function CurvePickerHost() {
  const request = useObservable(curvePickerRequest$, curvePickerRequest.value);
  // A new request is a new editor, so it starts from the curve it was asked with and not the one before.
  return request ? <CurvePickerDialog key={request.id} request={request} onDone={finishCurvePick} /> : null;
}
