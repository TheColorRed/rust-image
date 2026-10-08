import { app, BrowserWindow, dialog, Menu, type MenuItemConstructorOptions } from 'electron';
import { applyInstantAdjustment } from './actions/adjustments';
import { showDialog } from './dialog';
import { openProject } from './events/projects';
import { openFileDialog } from './native-dialogs';
// import { openProjectDialog } from './events/dialogs';

const isMac = process.platform === 'darwin';

export const menuTemplate: MenuItemConstructorOptions[] = [
  // App menu (macOS only)
  ...((isMac
    ? [
        {
          label: app.name,
          submenu: [
            { role: 'about' },
            { type: 'separator' },
            { role: 'services' },
            { type: 'separator' },
            { role: 'hide' },
            { role: 'hideOthers' },
            { role: 'unhide' },
            { type: 'separator' },
            { role: 'quit' },
          ],
        },
      ]
    : []) as MenuItemConstructorOptions[]),
  // File menu
  {
    label: 'File',
    submenu: [
      {
        label: 'Open...',
        accelerator: 'CmdOrCtrl+O',
        click: async () => {
          const files = (await openFileDialog()) ?? [];
          for (const file of files) {
            openProject(file);
          }
        },
      },
      { type: 'separator' },
      {
        label: 'Save',
        accelerator: 'CmdOrCtrl+S',
        click: () => {
          BrowserWindow.getFocusedWindow()?.webContents.send('menu-save');
        },
      },
      {
        label: 'Save As...',
        accelerator: 'CmdOrCtrl+Shift+S',
        click: async () => {
          const result = await dialog.showSaveDialog(BrowserWindow.getFocusedWindow()!, {
            filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
          });
          if (!result.canceled && result.filePath) {
            BrowserWindow.getFocusedWindow()!.webContents.send('menu-save-as', result.filePath);
          }
        },
      },
      { type: 'separator' },
      isMac ? { role: 'close' } : { role: 'quit' },
    ],
  },
  // Edit menu
  {
    label: 'Edit',
    submenu: [
      { role: 'undo' },
      { role: 'redo' },
      { type: 'separator' },
      { role: 'cut' },
      { role: 'copy' },
      { role: 'paste' },
      ...((isMac
        ? [{ role: 'pasteAndMatchStyle' }, { role: 'delete' }, { role: 'selectAll' }]
        : [{ role: 'delete' }, { type: 'separator' }, { role: 'selectAll' }]) as MenuItemConstructorOptions[]),
    ],
  },
  // Image Menu
  {
    label: 'Image',
    submenu: [
      {
        label: 'Adjustments',
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
  // Filters
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
  // View menu
  {
    label: 'View',
    submenu: [
      { role: 'reload' },
      { role: 'forceReload' },
      { role: 'toggleDevTools' },
      { type: 'separator' },
      { role: 'resetZoom' },
      { role: 'zoomIn' },
      { role: 'zoomOut' },
      { type: 'separator' },
      { role: 'togglefullscreen' },
    ],
  },
  // Window menu
  {
    label: 'Window',
    submenu: [
      { role: 'minimize' },
      { role: 'zoom' },
      ...((isMac
        ? [{ type: 'separator' }, { role: 'front' }, { type: 'separator' }, { role: 'window' }]
        : [{ role: 'close' }]) as MenuItemConstructorOptions[]),
    ],
  },
  // Help menu
  {
    label: 'Help',
    submenu: [
      {
        label: 'Learn More',
        click: async () => {
          const { shell } = await import('electron');
          await shell.openExternal('https://github.com/TheColorRed/rust-image');
        },
      },
    ],
  },
] as const;

export function createMenu(mainWindow: BrowserWindow): Menu {
  return Menu.buildFromTemplate(menuTemplate);
}
