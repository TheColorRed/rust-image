import { contextBridge, ipcRenderer } from 'electron';

export interface GradientOptions {
  start: [number, number];
  end: [number, number];
  type: 'linear' | 'radial';
  opacity: number;
  colorStart: string;
  colorEnd: string;
}

export interface AlakazamToolsApi {
  selection: {
    setArea: (projectId: string, area: [number, number][]) => void;
    setFeather: (projectId: string, feather: number) => void;
    clear: (projectId: string) => void;
  };
  gradient: {
    apply: (projectId: string, options: GradientOptions) => Promise<void>;
  };
}

contextBridge.exposeInMainWorld('tools', {
  selection: {
    setArea: (projectId: string, area: [number, number][]) =>
      ipcRenderer.invoke('tools-selection-set-area', projectId, area),
    setFeather: (projectId: string, feather: number) =>
      ipcRenderer.invoke('tools-selection-set-feather', projectId, feather),
    clear: (projectId: string) => ipcRenderer.invoke('tools-selection-clear', projectId),
  },
  gradient: {
    apply: (projectId: string, options: GradientOptions) =>
      ipcRenderer.invoke('tools-gradient-apply', projectId, options),
  },
} as AlakazamToolsApi);
