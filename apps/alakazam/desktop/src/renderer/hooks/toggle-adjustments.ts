import { useEffect, useRef } from 'react';

/**
 * Toggles the adjustment preview when the spacebar is pressed then restores it when released.
 * @param adjustmentName The name of the adjustment being toggled.
 * @param currentValues The current values of the adjustment.
 */
export function useToggleAdjustments<T extends Record<string, any>>(adjustmentName: string, currentValues: T) {
  const isSpacePressedRef = useRef(false);
  const valuesBeforeRef = useRef(currentValues);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === ' ' && !isSpacePressedRef.current) {
        isSpacePressedRef.current = true;
        valuesBeforeRef.current = currentValues;
        window.alakazam.adjustments.cancelAdjustment(adjustmentName);
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === ' ' && isSpacePressedRef.current) {
        isSpacePressedRef.current = false;
        window.alakazam.adjustments.previewAdjustment(adjustmentName, valuesBeforeRef.current);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
    };
  }, [adjustmentName, currentValues]);

  // Update valuesBefore when currentValues change, but only if not currently pressed
  useEffect(() => {
    if (!isSpacePressedRef.current) valuesBeforeRef.current = currentValues;
  }, [currentValues]);
}
