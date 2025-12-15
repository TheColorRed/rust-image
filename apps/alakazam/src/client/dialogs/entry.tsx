import '@/client/tailwind.css';
import React from 'react';
import { createRoot } from 'react-dom/client';

function Dialog({ children }: { children: React.ReactNode }) {
  return (
    <div data-name="dialog-root" className="bg-default text-white">
      {children}
    </div>
  );
}

/**
 * Dialog entrypoint — mounts the specific dialog component requested via the URL.
 * Example: http://localhost:8080/dialog.html?type=brightness-contrast
 */
async function main() {
  const params = new URLSearchParams(window.location.search);
  const dialogType = params.get('type') as DialogPath | null;
  const container = document.getElementById('root');
  if (!container) return;
  const root = createRoot(container);

  if (!dialogType) {
    root.render(React.createElement('div', null, 'No dialog type specified'));
    return;
  }

  let Component;
  // Adjustment dialogs
  if (dialogType === 'adjustments/brightness-contrast')
    Component = (await import('./adjustments/brightness-contrast')).BrightnessContrastDialog;
  else if (dialogType === 'adjustments/exposure') Component = (await import('./adjustments/exposure')).ExposureDialog;
  else if (dialogType === 'adjustments/vibrance') Component = (await import('./adjustments/vibrance')).VibranceDialog;
  // Filter blur dialogs
  else if (dialogType === 'blur/box-blur') Component = (await import('./blur/box-blur')).BoxBlurDialog;
  else if (dialogType === 'blur/gaussian-blur') Component = (await import('./blur/gaussian-blur')).GaussianBlurDialog;
  else if (dialogType === 'blur/lens-blur') Component = (await import('./blur/lens-blur')).LensBlurDialog;
  else if (dialogType === 'blur/motion-blur') Component = (await import('./blur/motion-blur')).MotionBlurDialog;
  else if (dialogType === 'blur/surface-blur') Component = (await import('./blur/surface-blur')).SurfaceBlurDialog;
  // Distort dialogs
  else if (dialogType === 'distort/pinch') Component = (await import('./distort/pinch')).PinchDialog;
  else if (dialogType === 'distort/ripple') Component = (await import('./distort/ripple')).RippleDialog;
  // Filter noise dialogs
  else if (dialogType === 'noise/add-noise') Component = (await import('./noise/add-noise')).AddNoiseDialog;
  else if (dialogType === 'noise/despeckle') Component = (await import('./noise/despeckle')).DespeckleDialog;
  else if (dialogType === 'noise/median') Component = (await import('./noise/median')).MedianDialog;
  // Unknown dialog
  else
    Component = () =>
      React.createElement(
        'div',
        {
          className: 'w-[400px] h-20 flex items-center justify-center',
        },
        React.createElement('h2', { className: 'text-xl font-semibold' }, `Unknown dialog: ${dialogType}`),
      );

  root.render(React.createElement(Dialog, null, React.createElement(Component)));
}

main();

export {};
