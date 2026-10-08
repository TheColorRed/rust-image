import { contextBridge, ipcRenderer } from 'electron';

export interface HueInfo {
  rgb: [number, number, number];
  offset: number;
  hue: number;
  hex: string;
}

export interface AlakazamGizmosApi {
  cursor: {
    circleCursor: (size: number) => Promise<ImageData>;
    arrowCursor: (size: number, direction: string, color: string) => Promise<ImageData>;
  };
  colorPicker: {
    colorPicker: (hue: string, width: number, height: number) => Promise<ImageData>;
    getColorAt: (color: string, x: number, y: number) => Promise<string>;
    getCursorPosition: (color: string, width: number, height: number) => Promise<[number, number]>;

    gradientHue: (width: number, height: number) => Promise<ImageData>;
    getHueAt: (y: number, height: number) => Promise<HueInfo>;
    getHueFrom: (color: string) => Promise<HueInfo>;
    getHuePosition: (hue: string, height: number) => Promise<number>;
  };
}

contextBridge.exposeInMainWorld('gizmos', {
  cursor: {
    circleCursor: (size: number) => ipcRenderer.invoke('gizmos-cursor-circle-cursor', size),
    arrowCursor: (size: number, direction: string, color: string) =>
      ipcRenderer.invoke('gizmos-cursor-arrow-cursor', size, direction, color),
  },
  colorPicker: {
    colorPicker: (hue: string, width: number, height: number) =>
      ipcRenderer.invoke('gizmos-color-picker', hue, width, height),
    getColorAt: (color: string, x: number, y: number) => ipcRenderer.invoke('gizmos-get-color-at', color, x, y),
    getCursorPosition: (color: string, width: number, height: number) =>
      ipcRenderer.invoke('gizmos-get-cursor-position', color, width, height),

    gradientHue: (width: number, height: number) => ipcRenderer.invoke('gizmos-hue-picker', width, height),
    getHueAt: (y: number, height: number) => ipcRenderer.invoke('gizmos-get-hue-offset', y, height),
    getHueFrom: (color: string) => ipcRenderer.invoke('gizmos-get-hue-from', color),
    getHuePosition: (hue: string, height: number) => ipcRenderer.invoke('gizmos-get-hue-position', hue, height),
  },
} as AlakazamGizmosApi);
