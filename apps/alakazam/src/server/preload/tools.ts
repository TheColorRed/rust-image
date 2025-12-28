import { contextBridge, ipcRenderer } from 'electron';

export interface AlakazamToolsApi {
  selection: {
    setArea: (projectId: string, area: [number, number][]) => void;
    setFeather: (projectId: string, feather: number) => void;
  };
}

contextBridge.exposeInMainWorld('tools', {
  selection: {
    setArea: (projectId: string, area: [number, number][]) =>
      ipcRenderer.invoke('tools-selection-set-area', projectId, area),
    setFeather: (projectId: string, feather: number) =>
      ipcRenderer.invoke('tools-selection-set-feather', projectId, feather),
  },
} as AlakazamToolsApi);
