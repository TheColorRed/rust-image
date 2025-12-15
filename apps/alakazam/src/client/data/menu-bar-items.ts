export interface MenuItem {
  /** The label of the menu item. */
  label?: string;
  /** The type of the menu item. */
  type?: 'separator';
  /** Whether the menu item is disabled. */
  disabled?: boolean;
  /** Whether the menu item requires an active layer to be enabled. */
  requiresLayer?: boolean;
  /** The click handler for the menu item. */
  click?: () => void;
  /** The submenu items, if any. */
  submenu?: MenuItem[];
}
/**
 * Shows a dialog window with the specified type and title.
 * @param dialogPath The path of the dialog to show, in the format 'feature/type'.
 * @param title The title of the dialog window.
 */
function showDialog(dialogPath: DialogPath, title: string) {
  window.alakazam.showDialog(dialogPath, title);
}

function applyInstantAdjustment(adjustmentType: 'autoColor' | 'autoTone' | 'invert' | 'grayscale') {
  window.alakazam.adjustments.applyInstantAdjustment(adjustmentType);
}

export const menubarItems: MenuItem[] = [
  {
    label: 'File',
    submenu: [
      {
        label: 'Open...',
        click: async () => {
          const files = await window.alakazam.openFileDialog(['multiSelections']);
          for (const file of files ?? []) window.alakazam.projects.openProject(file);
        },
      },
    ],
  },
  {
    label: 'Edit',
    submenu: [
      {
        label: 'Open...',
      },
    ],
  },
  {
    label: 'Image',
    submenu: [
      {
        label: 'Adjustments',
        requiresLayer: true,
        submenu: [
          {
            label: 'Brightness/Contrast...',
            click: () => showDialog('adjustments/brightness-contrast', 'Brightness/Contrast'),
          },
          {
            label: 'Exposure...',
            click: () => showDialog('adjustments/exposure', 'Exposure'),
          },
          {
            label: 'Vibrance...',
            click: () => showDialog('adjustments/vibrance', 'Vibrance'),
          },
        ],
      },
      { type: 'separator' },
      {
        label: 'Auto Color',
        click: () => applyInstantAdjustment('autoColor'),
      },
      {
        label: 'Auto Tone',
        click: () => applyInstantAdjustment('autoTone'),
      },
      {
        label: 'Invert',
        click: () => applyInstantAdjustment('invert'),
      },
      {
        label: 'Grayscale',
        click: () => applyInstantAdjustment('grayscale'),
      },
    ],
  },
  {
    label: 'Filters',
    submenu: [
      {
        label: 'Blur',
        submenu: [
          { label: 'Box Blur...', click: () => showDialog('blur/box-blur', 'Box Blur') },
          { label: 'Gaussian Blur...', click: () => showDialog('blur/gaussian-blur', 'Gaussian Blur') },
          { label: 'Lens Blur...', click: () => showDialog('blur/lens-blur', 'Lens Blur') },
          { label: 'Motion Blur...', click: () => showDialog('blur/motion-blur', 'Motion Blur') },
          { label: 'Surface Blur...', click: () => showDialog('blur/surface-blur', 'Surface Blur') },
        ],
      },
      {
        label: 'Distort',
        submenu: [
          { label: 'Pinch...', click: () => showDialog('distort/pinch', 'Pinch') },
          { label: 'Ripple...', click: () => showDialog('distort/ripple', 'Ripple') },
        ],
      },
      {
        label: 'Noise',
        submenu: [
          { label: 'Add Noise...', click: () => showDialog('noise/add-noise', 'Noise') },
          { label: 'Despeckle...', click: () => showDialog('noise/despeckle', 'Despeckle') },
          { label: 'Median...', click: () => showDialog('noise/median', 'Median') },
        ],
      },
    ],
  },
];
