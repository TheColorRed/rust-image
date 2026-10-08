export type CommandPrefix = 'CmdOrCtrl' | 'Alt' | 'Shift';
export type CommandKey = `${CommandPrefix}+${string}`;

export interface CommandAction {
  /** A stable id for the action. Use this so HMR can update handlers. */
  id: string;
  /** The keyboard shortcut for the command action. */
  key?: CommandKey;
  /** An optional context where the command action is applicable. */
  where?: string | string[];
  /** Whether the command action is enabled. */
  enabled?: boolean;
  /** Legacy: A callback function to be executed when the command action is triggered. */
  callback?: (...args: any[]) => void;
}

// Use a single global storage on `window` so HMR module reloads don't leave
// multiple competing state copies. This prevents race conditions where one
// module clears accelerators and another re-registers them.
const GLOBAL_KEY = '__alakazam_accelerator_v1';

type GlobalState = {
  accelerators: Array<Pick<CommandAction, 'key' | 'id' | 'enabled'>>;
  handlers: Map<string, () => void>;
};

const globalAny = window as any;
if (!globalAny[GLOBAL_KEY]) {
  globalAny[GLOBAL_KEY] = {
    accelerators: [],
    handlers: new Map<string, () => void>(),
  } as GlobalState;
}

const getState = (): GlobalState => globalAny[GLOBAL_KEY];

/**
 * Adds a new command action to the accelerators list.
 * If an `id` and `callback` are provided they are registered into the handler registry.
 */
export const addAccelerator = (action: CommandAction) => {
  const state = getState();
  // Remove any existing accelerator that has the same key or id
  state.accelerators = state.accelerators.filter(a => a.key !== action.key && (action.id ? a.id !== action.id : true));

  state.accelerators.push({
    key: action.key,
    id: action.id,
    enabled: action.enabled ?? true,
  });

  if (action.id && action.callback) state.handlers.set(action.id, action.callback);

  // For easy debugging
  console.log('addAccelerator:', action.key, 'id=', action.id, 'total=', state.accelerators.length);
};

/**
 * Updates an existing command action or key in the accelerators list.
 */
export const updateAccelerator = (oldAction: CommandAction | CommandKey, newAction: Partial<CommandAction>) => {
  const state = getState();
  let cmdIndex = -1;
  if (typeof oldAction === 'string') cmdIndex = state.accelerators.findIndex(i => i.key === oldAction);
  else cmdIndex = state.accelerators.findIndex(i => i.id && oldAction.id === i.id);
  if (cmdIndex !== -1) state.accelerators[cmdIndex] = { ...state.accelerators[cmdIndex], ...newAction } as any;
  if (newAction.id && newAction.callback) {
    state.handlers.set(newAction.id, newAction.callback);
  }
};

/**
 * Removes a command action or key from the accelerators list.
 */
export const removeAccelerator = (action: CommandAction | CommandKey) => {
  const state = getState();
  let cmdIndex = -1;
  if (typeof action === 'string') cmdIndex = state.accelerators.findIndex(i => i.key === action);
  else cmdIndex = state.accelerators.findIndex(i => i.id && action.id === i.id);
  if (cmdIndex !== -1) state.accelerators.splice(cmdIndex, 1);
  if (typeof action !== 'string' && action.id) state.handlers.delete(action.id);
};

/**
 * Clears all command actions and handlers.
 */
export const clearAccelerators = () => {
  const state = getState();
  state.accelerators = [];
  state.handlers.clear();
};

/** Whether the platform is macOS. */
export const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;

export const acceleratorDisplayName = (key: CommandKey) => {
  return key
    .replace(/CmdOrCtrl/g, isMac ? '⌘' : 'Ctrl')
    .replace(/Alt/g, isMac ? '⌥' : 'Alt')
    .replace(/Shift/g, isMac ? '⇧' : 'Shift')
    .replace(/\+/g, ' + ');
};

const handleKeyDown = (event: KeyboardEvent) => {
  const pressedKeys: string[] = [];
  if (event.ctrlKey || (isMac && event.metaKey)) pressedKeys.push('CmdOrCtrl');
  if (event.altKey) pressedKeys.push('Alt');
  if (event.shiftKey) pressedKeys.push('Shift');
  pressedKeys.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
  const pressedCombination = pressedKeys.join('+');
  console.log('Pressed combination:', pressedCombination);

  const state = getState();
  const command = state.accelerators.find(cmd => cmd.key === pressedCombination && (cmd.enabled ?? true));
  console.log('Matched command:', command);
  console.log('Accelerators list:', state.accelerators.length);

  if (!command) return;

  // Resolve the handler at invocation time so HMR-updated handlers are used.
  if (command.id) {
    const handler = state.handlers.get(command.id);
    if (handler) {
      handler();
      return;
    }
  }

  // Backwards compat: if a callback was provided directly, call it (rare)
  // @ts-ignore (compat)
  command.callback?.();
};

// Ensure we only ever add ONE keydown listener across HMR replacements.
if (globalAny.__alakazam_handleKeyDown) {
  window.removeEventListener('keydown', globalAny.__alakazam_handleKeyDown);
}
globalAny.__alakazam_handleKeyDown = handleKeyDown;
window.addEventListener('keydown', handleKeyDown);

// HMR cleanup (best-effort; supports both webpack-style `module.hot` and `import.meta.hot`)
if ((module as any)?.hot?.dispose) {
  (module as any).hot.dispose(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });
}
if ((import.meta as any)?.hot?.dispose) {
  (import.meta as any).hot.dispose(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });
}
