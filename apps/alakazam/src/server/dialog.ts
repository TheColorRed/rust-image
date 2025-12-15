import { BrowserWindow, ipcMain, webContents } from 'electron';
import path from 'path';
import { auditTime, Subject } from 'rxjs';
import { fileURLToPath } from 'url';
import { cancelAdjustment } from './actions/adjustments.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export interface DialogOptions {
  size?: [number, number];
  blockParent?: boolean;
}

const consoleMessage = new Subject<{ message: string; level: number; line: number; sourceId: string }>();
consoleMessage.pipe(auditTime(250)).subscribe(message => {
  // Send the console messages to the main window's console using ipc
  webContents.getAllWebContents().forEach(wc => wc.send('dialog-console-message', message));
});

export function showDialog(
  dialogType: DialogPath,
  title: string,
  options: DialogOptions = {
    size: [0, 0],
    blockParent: true,
  },
) {
  const size = options.size ?? [0, 0];
  const focusedWindow = BrowserWindow.getFocusedWindow();
  if (focusedWindow) {
    const browserWindow = new BrowserWindow({
      parent: focusedWindow,
      modal: options.blockParent ?? true,
      width: size[0],
      height: size[1],
      resizable: false,
      minimizable: false,
      maximizable: false,
      skipTaskbar: true,
      autoHideMenuBar: true,
      show: false,
      webPreferences: {
        preload: path.join(__dirname, 'preload.js'),
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
      },
    });
    browserWindow.setMenu(null);
    browserWindow.loadURL(`http://localhost:8080/dialog.html?type=${encodeURIComponent(dialogType)}`);
    browserWindow.setTitle(title ?? 'Alakazam');

    // Show developer tools if in development mode
    // if (process.env.NODE_ENV === 'development') browserWindow.webContents.openDevTools({ mode: 'detach' });

    function calculateWindowSize() {
      const CALCULATE_WINDOW_SIZE = `new Promise((resolve) => {
              // Wait for dialog-root element to be available
              const waitForElement = () => {
                const dialogRoot = document.querySelector('[data-name="dialog-root"]');

                if (dialogRoot) {
                  // Give React time to fully update the DOM
                  setTimeout(() => {
                    const body = document.body;

                    // Store original styles
                    const originalHeight = dialogRoot.style.height;
                    const originalWidth = dialogRoot.style.width;
                    const originalRootOverflow = dialogRoot.style.overflow;
                    const originalBodyOverflow = body.style.overflow;

                    // Temporarily remove size constraints and hide scrollbars to measure natural content size
                    dialogRoot.style.height = 'auto';
                    dialogRoot.style.width = 'auto';
                    dialogRoot.style.overflow = 'hidden';
                    body.style.overflow = 'hidden';

                    // Force reflow to get accurate dimensions
                    void body.offsetHeight;

                    // Find the actual content wrapper (first child with w-[400px])
                    const contentWrapper = dialogRoot.querySelector('[class*="w-["]');

                    // Measure the actual content size
                    const rect = (contentWrapper || dialogRoot).getBoundingClientRect();
                    let width = Math.ceil(rect.width);
                    let height = Math.ceil(rect.height);

                    // Restore original width/height, but keep overflow hidden until main restores it
                    dialogRoot.style.height = originalHeight;
                    dialogRoot.style.width = originalWidth;

                    // Fallback to body dimensions if measured size is invalid
                    if (width === 0 || height === 0) {
                      width = body.scrollWidth || 300;
                      height = body.scrollHeight || 200;
                    }

                    resolve({
                      width: width + 15,
                      height: height,
                      originalRootOverflow: originalRootOverflow || '',
                      originalBodyOverflow: originalBodyOverflow || ''
                    });
                  }, 50);
                } else {
                  // Element not found yet, check again on next frame
                  requestAnimationFrame(waitForElement);
                }
              };

              waitForElement();
            });`;
      return browserWindow.webContents.executeJavaScript(CALCULATE_WINDOW_SIZE, true);
    }

    let overflowRestoreTimer: NodeJS.Timeout | undefined;

    const handleUpdateDialogSize = async (event: Electron.IpcMainInvokeEvent) => {
      // Only handle if this event is from our specific window
      if (event.sender === browserWindow.webContents) {
        const dimensions: any = await calculateWindowSize();
        let width = dimensions.width;
        const titleBarHeight = browserWindow.getSize()[1] - browserWindow.getContentSize()[1];
        const wasVisible = browserWindow.isVisible();

        // Temporarily make window resizable to allow setSize to work
        browserWindow.setResizable(true);
        browserWindow.setSize(width, dimensions.height + titleBarHeight);
        browserWindow.setResizable(false);

        if (!wasVisible) browserWindow.show();

        // After resizing, restore overflow to original values (no scrollbar flicker)
        if (overflowRestoreTimer) clearTimeout(overflowRestoreTimer);
        overflowRestoreTimer = setTimeout(() => {
          const rootOverflow = dimensions.originalRootOverflow || '';
          const bodyOverflow = dimensions.originalBodyOverflow || '';
          const RESTORE_OVERFLOW = `(() => {
            const r = document.querySelector('[data-name="dialog-root"]');
            if (r) r.style.overflow = ${JSON.stringify(rootOverflow)};
            document.body.style.overflow = ${JSON.stringify(bodyOverflow)};
          })()`;
          browserWindow.webContents.executeJavaScript(RESTORE_OVERFLOW, true).catch(() => {});
        }, 150);
      }
    };

    // Remove any existing handler first to avoid conflicts
    ipcMain.removeHandler('update-dialog-window-size');
    ipcMain.handle('update-dialog-window-size', handleUpdateDialogSize);

    if (size[0] === 0 && size[1] === 0) {
      // resize the window to fit content on first load
      browserWindow.webContents.once('did-finish-load', async () => {
        try {
          const dimensions: any = await calculateWindowSize();
          const width = dimensions.width;
          const titleBarHeight = browserWindow.getSize()[1] - browserWindow.getContentSize()[1];
          const wasVisible = browserWindow.isVisible();

          browserWindow.setResizable(true);
          browserWindow.setSize(width, dimensions.height + titleBarHeight);
          browserWindow.setResizable(false);
          if (!wasVisible) browserWindow.show();
          if (focusedWindow) browserWindow.center();

          // Restore overflow after a short delay
          const rootOverflow = dimensions.originalRootOverflow || '';
          const bodyOverflow = dimensions.originalBodyOverflow || '';
          setTimeout(() => {
            const RESTORE_OVERFLOW = `(() => { const r = document.querySelector('[data-name="dialog-root"]'); if (r) r.style.overflow = ${JSON.stringify(rootOverflow)}; document.body.style.overflow = ${JSON.stringify(bodyOverflow)}; })()`;
            browserWindow.webContents.executeJavaScript(RESTORE_OVERFLOW, true).catch(() => {});
          }, 150);
        } catch (error) {
          console.error('Error calculating initial window size:', error);
          if (!browserWindow.isVisible()) browserWindow.show();
        }
      });
    } else {
      browserWindow.once('ready-to-show', () => {
        browserWindow.show();
        // Center the dialog over the parent window
        if (focusedWindow) browserWindow.center();
      });
    }
    browserWindow.on('closed', () => {
      ipcMain.removeHandler('update-dialog-window-size');
      if (typeof overflowRestoreTimer !== 'undefined') clearTimeout(overflowRestoreTimer);
      cancelAdjustment();
    });

    // Pipe console messages from the dialog window to the main window for easier debugging.
    browserWindow.webContents.on('console-message', (event, level, message, line, sourceId) => {
      consoleMessage.next({ message, level, line, sourceId });
    });
  }
}
