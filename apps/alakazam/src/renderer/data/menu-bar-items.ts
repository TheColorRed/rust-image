import { CommandId } from '@/data/commands';
import type { CommandPrefix } from '@/services/accelerator';

export interface MenuItem {
  /** The label of the menu item. */
  label?: string;
  /** The type of the menu item. */
  type?: 'separator';
  /** Whether the menu item is disabled. */
  disabled?: boolean;
  /** Whether the menu item requires an active layer to be enabled. */
  requiresLayer?: boolean;
  /** The keyboard shortcut for the menu item. */
  accelerator?: `${CommandPrefix}+${string}`;
  /** The click handler for the menu item. */
  click?: () => void;
  /** The command associated with the menu item. */
  command?: CommandId;
  /** Optional arguments to pass to the command, if any. */
  commandArgs?: any[];
  /** The submenu items, if any. */
  submenu?: MenuItem[];
}

export const menubarItems: MenuItem[] = [
  {
    label: 'File',
    submenu: [
      {
        label: 'Open...',
        command: 'open-file',
      },
      {
        label: 'Export As...',
        accelerator: 'CmdOrCtrl+Shift+E',
        command: 'export-as',
      },
    ],
  },
  {
    label: 'Edit',
    submenu: [
      { label: 'Undo', command: 'undo' },
      { label: 'Redo', command: 'redo' },
      { type: 'separator' },
      { label: 'Cut', accelerator: 'CmdOrCtrl+X' },
      { label: 'Copy', accelerator: 'CmdOrCtrl+C' },
      { label: 'Paste', accelerator: 'CmdOrCtrl+V' },
      { type: 'separator' },
      { label: 'Fill...' },
      { label: 'Stroke...' },
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
            command: 'show-dialog',
            commandArgs: ['adjustments/brightness-contrast', 'Brightness/Contrast'],
          },
          {
            label: 'Exposure...',
            command: 'show-dialog',
            commandArgs: ['adjustments/exposure', 'Exposure'],
          },
          {
            label: 'Vibrance...',
            command: 'show-dialog',
            commandArgs: ['adjustments/vibrance', 'Vibrance'],
          },
        ],
      },
      { type: 'separator' },
      {
        label: 'Auto Color',
        command: 'instant-adjustment',
        commandArgs: ['autoColor'],
      },
      {
        label: 'Auto Tone',
        command: 'instant-adjustment',
        commandArgs: ['autoTone'],
      },
      {
        label: 'Invert',
        command: 'instant-adjustment',
        commandArgs: ['invert'],
      },
      {
        label: 'Grayscale',
        command: 'instant-adjustment',
        commandArgs: ['grayscale'],
      },
    ],
  },
  {
    label: 'Filters',
    submenu: [
      {
        label: 'Blur',
        submenu: [
          { label: 'Box Blur...', command: 'show-dialog', commandArgs: ['blur/box-blur', 'Box Blur'] },
          { label: 'Gaussian Blur...', command: 'show-dialog', commandArgs: ['blur/gaussian-blur', 'Gaussian Blur'] },
          { label: 'Lens Blur...', command: 'show-dialog', commandArgs: ['blur/lens-blur', 'Lens Blur'] },
          { label: 'Motion Blur...', command: 'show-dialog', commandArgs: ['blur/motion-blur', 'Motion Blur'] },
          { label: 'Surface Blur...', command: 'show-dialog', commandArgs: ['blur/surface-blur', 'Surface Blur'] },
        ],
      },
      {
        label: 'Distort',
        submenu: [
          { label: 'Pinch...', command: 'show-dialog', commandArgs: ['distort/pinch', 'Pinch'] },
          { label: 'Ripple...', command: 'show-dialog', commandArgs: ['distort/ripple', 'Ripple'] },
        ],
      },
      {
        label: 'Noise',
        submenu: [
          { label: 'Add Noise...', command: 'show-dialog', commandArgs: ['noise/add-noise', 'Noise'] },
          { label: 'Despeckle...', command: 'show-dialog', commandArgs: ['noise/despeckle', 'Despeckle'] },
          { label: 'Median...', command: 'show-dialog', commandArgs: ['noise/median', 'Median'] },
        ],
      },
    ],
  },
];
