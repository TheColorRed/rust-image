import { clipboard, ipcMain, nativeImage } from 'electron';

function fixPixels(bitmap: Uint8Array) {
  let arr = new Uint8Array(bitmap.length);
  for (let i = 0; i < bitmap.length; i += 4) {
    arr[i + 0] = bitmap[i + 2];
    arr[i + 1] = bitmap[i + 1];
    arr[i + 2] = bitmap[i + 0];
    arr[i + 3] = bitmap[i + 3];
  }
  return arr;
}

ipcMain.handle('clipboard-write-pixels', async (_event, pixels: Uint8Array, width: number, height: number) => {
  const image = nativeImage.createFromBuffer(Buffer.from(fixPixels(pixels)), { width, height });
  clipboard.writeImage(image);
});

ipcMain.handle('clipboard-read-image', async () => {
  const image = clipboard.readImage();
  if (image.isEmpty()) return null;

  const bitmap = image.toBitmap();
  const size = image.getSize();
  return { data: Uint8Array.from(fixPixels(bitmap)), width: size.width, height: size.height };
});
