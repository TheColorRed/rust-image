import { contextBridge, ipcRenderer } from 'electron';

export interface AlakazamClipboardApi {
  writePixels: (pixels: ImageDataArray, width: number, height: number) => Promise<void>;
  readPixels: () => Promise<{ data: Uint8Array; width: number; height: number } | null>;
}

contextBridge.exposeInMainWorld('clipboard', {
  writePixels: async (pixels: ImageDataArray, width: number, height: number) =>
    ipcRenderer.invoke('clipboard-write-pixels', pixels, width, height),
  readPixels: async () => ipcRenderer.invoke('clipboard-read-image'),
} as AlakazamClipboardApi);
