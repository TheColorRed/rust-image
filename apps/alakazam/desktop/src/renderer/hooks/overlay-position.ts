import { RefObject, useCallback } from 'react';

/**
 * Placement of the overlay relative to the trigger element.
 * - `above`: Overlay is positioned above the trigger.
 * - `below`: Overlay is positioned below the trigger.
 * - `center`: Overlay is centered vertically with respect to the trigger.
 */
export type Placement = 'above' | 'below' | 'center';
/**
 * Adjustment for the overlay position relative to the trigger element.
 * - `center`: Center the overlay horizontally with respect to the trigger.
 * - `left`: Align the right side of the overlay with the left side of the trigger.
 * - `right`: Align the left side of the overlay with the right side of the trigger.
 * - `start`: Align the left side of the overlay with the left side of the trigger.
 * - `end`: Align the right side of the overlay with the right side of the trigger.
 */
export type Adjustment = 'left' | 'center' | 'right' | 'start' | 'end';

export function useOverlayPosition() {
  return useCallback(
    (
      /** The reference to the trigger element that the overlay is positioned relative to. */
      triggerRef: RefObject<HTMLElement | null> | HTMLElement | null,
      /** The reference to the overlay element to be positioned. */
      overlayRef: RefObject<HTMLElement | null> | HTMLElement | null,
      /** The placement of the overlay along the y-axis. */
      placement: Placement,
      /** The adjustment of the overlay along the x-axis. */
      adjustment: Adjustment,
    ) => {
      if (overlayRef instanceof HTMLElement) overlayRef = { current: overlayRef };
      if (triggerRef instanceof HTMLElement) triggerRef = { current: triggerRef };

      if (!overlayRef || !triggerRef || !overlayRef.current || !triggerRef.current)
        return { top: 0, left: 0, width: 0 };
      const overlayRect = overlayRef.current.getBoundingClientRect();
      const triggerRect = triggerRef.current.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let top =
        placement === 'above'
          ? triggerRect.top - overlayRect.height
          : placement === 'below'
            ? triggerRect.bottom
            : triggerRect.top + triggerRect.height / 2 - overlayRect.height / 2;

      let left =
        adjustment === 'center'
          ? triggerRect.left + triggerRect.width / 2 - overlayRect.width / 2
          : adjustment === 'right'
            ? triggerRect.right
            : adjustment === 'left'
              ? triggerRect.left - overlayRect.width
              : adjustment === 'start'
                ? triggerRect.left
                : adjustment === 'end'
                  ? triggerRect.right - overlayRect.width
                  : triggerRect.left;

      // If centered and it overflows to the right, clamp to viewport
      if (adjustment === 'center' && left + overlayRect.width > viewportWidth) left = viewportWidth - overlayRect.width;

      // If it overflows to the right, prefer to place to the left of the trigger when the desired
      // adjustment was `right` (tooltip to the right) — this avoids overlapping the trigger.
      if (left + overlayRect.width > viewportWidth) {
        if (adjustment === 'right') {
          // Place overlay to the left of the trigger and leave a small gap so it doesn't overlap
          const horizontalGap = 8; // same as tooltip offset
          left = triggerRect.left - overlayRect.width - 2 * horizontalGap; // account for tooltip offset + gap
        } else {
          // Otherwise clamp to viewport
          left = viewportWidth - overlayRect.width;
        }
      }

      // If centered and it overflows to the left, try to preserve the overlay's previous position
      if (adjustment === 'center' && left < 0) left = 0;

      // If it still overflows to the left, prefer to place to the right of the trigger when the
      // desired adjustment was `left` (tooltip to the left). This avoids overlapping the trigger.
      if (left < 0) {
        if (adjustment === 'left') {
          const horizontalGap = 8; // same as tooltip offset
          left = triggerRect.right + 2 * horizontalGap;
        } else {
          // As a final fallback clamp to 0
          left = 0;
        }
      }

      // Move the menu above the trigger if it overflows below
      if (top + overlayRect.height > viewportHeight) top = triggerRect.top - overlayRect.height;
      // Move the menu below the trigger if it overflows above
      if (top < 0) top = triggerRect.bottom;

      return {
        top,
        left,
        width: overlayRect.width,
      };
    },
    [],
  );
}
