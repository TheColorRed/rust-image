import { HueInfo } from '@/preload/gizmos';
import { ipcMain } from 'electron';

ipcMain.handle('gizmos-cursor-circle-cursor', async (event, size: number) => {
  const imageData = gizmos.circleCursor(size);
  return imageData;
});

ipcMain.handle('gizmos-cursor-arrow-cursor', async (event, size: number, direction: string, color: string) => {
  const imageData = gizmos.arrowCursor(size, direction, color);
  return imageData;
});

ipcMain.handle('gizmos-color-picker', async (event, hue: string, width: number, height: number) => {
  const color = abra.Color.fromHexString(hue);
  const imageData = gizmos.colorPicker(color, width, height);
  return imageData;
});
ipcMain.handle('gizmos-hue-picker', async (event, width: number, height: number) => {
  const imageData = gizmos.gradientHue(width, height);
  return imageData;
});
ipcMain.handle('gizmos-get-color-at', async (event, hue: string, x: number, y: number) => {
  const color = abra.Color.fromHexString(hue);
  const result = gizmos.getColorAt(color.hsv()[0], x, y);
  return result.toHexString();
});
ipcMain.handle('gizmos-get-cursor-position', async (event, hue: string, width: number, height: number) => {
  const color = abra.Color.fromHexString(hue);
  const result = gizmos.getColorPickerCursorPosition(color, width, height);
  return [result[0], result[1]];
});
ipcMain.handle('gizmos-get-hue-offset', async (event, y: number, height: number): Promise<HueInfo> => {
  const result = gizmos.getHueAt(y, height);
  return {
    rgb: [
      abra.Color.fromHexString(result.hex).r,
      abra.Color.fromHexString(result.hex).g,
      abra.Color.fromHexString(result.hex).b,
    ],
    hue: result.hue,
    hex: result.hex,
    offset: result.offset,
  };
});
ipcMain.handle('gizmos-get-hue-from', async (event, colorHex: string): Promise<HueInfo> => {
  const color = abra.Color.fromHexString(colorHex);
  const hueValue = color.hsv()[0];
  const hueColor = abra.Color.fromHsv(hueValue, 1, 1);
  return {
    rgb: [hueColor.r, hueColor.g, hueColor.b],
    hue: hueValue,
    hex: hueColor.toHexString(),
    offset: hueValue / 360,
  };
});
ipcMain.handle('gizmos-get-hue-position', async (event, hue: string, height: number) => {
  const color = abra.Color.fromHexString(hue);
  const result = gizmos.getHueCursorPosition(color.hsv()[0], height);
  console.log('result', result, `${color.hsv()[0]}, ${height}`);
  return result;
});
