import { addAccelerator, clearAccelerators, type CommandAction } from '@/services/accelerator';
import { InstantAdjustmentType } from '@server/preload/alakazam';

/** The union type of all command IDs defined in the commands array. */
export type CommandId = (typeof commands)[number]['id'];

/**
 * Runs a command by its ID.
 * @param id The ID of the command to run.
 * @param args Optional arguments to pass to the command.
 * @returns The result of the command callback, if any.
 */
export function runCommand(id: CommandId, ...args: any[]) {
  const command = getCommand(id);
  if (command && command.callback) return command.callback(...args);
  else console.warn(`Command with id "${id}" not found or has no callback.`);
}
/**
 * Gets a command by its ID.
 * @param id The ID of the command to retrieve.
 * @returns The command action or undefined if not found.
 */
export function getCommand(id: CommandId) {
  return commands.find(cmd => cmd.id === id) as CommandAction | undefined;
}

export const commands = [
  // File Menu
  {
    id: 'open-file',
    key: 'CmdOrCtrl+O',
    callback: async () => {
      const files = await window.alakazam.openFileDialog('Open Image', ['multiSelections']);
      for (const file of files ?? []) window.alakazam.projects.openProject(file);
    },
  },
  {
    id: 'export-as',
    key: 'CmdOrCtrl+Shift+E',
    callback: async () => {
      const file = await window.alakazam.saveFileDialog('Export As...');
      console.log(file);
      if (file) window.alakazam.projects.exportActiveProject(file);
    },
  },
  // Edit Menu
  {
    id: 'undo',
    key: 'CmdOrCtrl+Z',
    callback: () => {
      // window.alakazamHistory.undo();
    },
  },
  {
    id: 'redo',
    key: 'Shift+CmdOrCtrl+Z',
    callback: () => {
      // window.alakazamHistory.redo();
    },
  },
  {
    id: 'copy',
    key: 'CmdOrCtrl+C',
    callback: async () => {
      const project = await window.alakazam.projects.getActiveProjectMetadata();
      if (!project) return;
      const area = [0, 0, 500, 500] as [number, number, number, number]; // TODO: Get actual selection area
      const pixels = await window.alakazam.imageData.getPixels(project.id, area);
      if (pixels) window.clipboard.writePixels(pixels.data, pixels.width, pixels.height);
    },
  },
  {
    id: 'paste',
    key: 'CmdOrCtrl+V',
    callback: async () => {
      const project = await window.alakazam.projects.getActiveProjectMetadata();
      if (!project) return;
      await window.alakazam.projects.pasteImageFromClipboard(project.id);
    },
  },

  // Instant Adjustments
  {
    id: 'instant-adjustment',
    callback: (adjustmentType: InstantAdjustmentType) => {
      window.alakazam.adjustments.applyInstantAdjustment(adjustmentType);
    },
  },
  // Dialogs
  {
    id: 'show-dialog',
    callback: (dialogPath: DialogPath, title: string) => {
      window.alakazam.showDialog(dialogPath, title);
    },
  },

  // Dev commands
  {
    id: 'dev-reload-renderer',
    callback: () => window.location.reload(),
  },
  {
    id: 'dev-toggle-devtools',
    callback: () => window.alakazam.developer.toggleDevTools(),
  },
] as const satisfies CommandAction[];

function registerCommands(items: CommandAction[], path: string[] = []) {
  items.forEach(item => {
    const id = [...path, item.id].filter(Boolean).join('/');
    if (item.key && item.callback) {
      addAccelerator({
        key: item.key,
        id,
        callback: item.callback,
      });
    }
  });
  // items.forEach(item => {
  //   const id = [...path, item.label ?? ''].filter(Boolean).join('/');
  //   if (item.submenu) registerCommands(item.submenu, [...path, item.label ?? '']);
  //   if (item.accelerator && item.click) {
  //     addAccelerator({
  //       key: item.accelerator,
  //       id,
  //       callback: item.click,
  //     });
  //   }
  // });
}

registerCommands(commands);

if (process.env.NODE_ENV === 'development') {
  // listen for webpack hot reloads to re-register menu items
  // remove existing listener if present (avoid duplicates across HMR)
  const existing = (window as any).__alakazam_menu_hmr_listener;
  if (existing) window.removeEventListener('message', existing);

  const messageHandler = ({ data }: MessageEvent) => {
    if (data.type === 'webpackInvalid') {
      console.log('Hot reloading menu items...');
      clearAccelerators();
    }
    if (data.type === 'webpackOk') {
      console.log('Re-registering menu items...');
      // clear then re-register after a microtask so module re-exec settles
      clearAccelerators();
      setTimeout(() => registerCommands(commands), 0);
    }
  };

  window.addEventListener('message', messageHandler);
  (window as any).__alakazam_menu_hmr_listener = messageHandler;

  // HMR cleanup
  if ((module as any)?.hot?.dispose) {
    (module as any).hot.dispose(() => window.removeEventListener('message', messageHandler));
  }
  if ((import.meta as any)?.hot?.dispose) {
    (import.meta as any).hot.dispose(() => window.removeEventListener('message', messageHandler));
  }
}
