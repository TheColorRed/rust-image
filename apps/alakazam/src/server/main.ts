import { menuTemplate } from '@/main-menu';
import { app, BrowserWindow, ipcMain, Menu, session } from 'electron';
import fs from 'fs';
import { createRequire } from 'module';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

function resolvePreload() {
  const candidates = [
    path.join(__dirname, 'preload.js'),
    path.join(process.cwd(), 'dist', 'server', 'preload.js'),
    path.join(__dirname, '..', '..', 'dist', 'server', 'preload.js'),
    path.join(__dirname, '..', 'dist', 'server', 'preload.js'),
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  // Fallback to the builtin location — Electron will report a clear error if it's missing
  return path.join(__dirname, 'preload.js');
}

const isDev = process.env.NODE_ENV === 'development';

// Suppress security warning in development (unsafe-eval is required for webpack HMR)
if (isDev) {
  process.env.ELECTRON_DISABLE_SECURITY_WARNINGS = 'true';
}

declare global {
  var abra: typeof import('@alakazam/abra');
  var alakazamHistory: typeof import('@alakazam/history');
}

try {
  global.abra = require('@alakazam/abra/abra.node');
  global.alakazamHistory = require('@alakazam/history/alakazam-history.node');
  console.log('Native module loaded successfully');
} catch (err) {
  console.error('Failed to load native module:', err);
  process.exit(1);
}

const reactDevToolsPath = path.join(
  os.homedir(),
  'AppData/Local/Google/Chrome/User Data/Default/Extensions/fmkadmapgofadopljbjfkapdkoienihi/7.0.1_0',
);

app.whenReady().then(async () => {
  await session.defaultSession.extensions.loadExtension(reactDevToolsPath, { allowFileAccess: true });
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          isDev
            ? "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:; img-src 'self' data: blob:; connect-src 'self' ws://localhost:8080"
            : "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com data:; img-src 'self' data: blob:",
        ],
      },
    });
  });

  console.log('Resolved preload path:', resolvePreload());
  const mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    titleBarStyle: 'hidden',
    autoHideMenuBar: true,
    webPreferences: {
      preload: resolvePreload(),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  // Create and set the application menu
  // const menu = createMenu(mainWindow);
  // Menu.setApplicationMenu(menu);
  mainWindow.maximize();
  mainWindow.moveTop();

  // Workaround to ensure window is on top on launch
  mainWindow.setAlwaysOnTop(true);
  mainWindow.flashFrame(false);
  mainWindow.once('ready-to-show', () => mainWindow.setAlwaysOnTop(false));

  // For testing: Open a project on launch
  if (isDev) {
    const project = openProject('C:/Users/untun/Documents/vscode/rust/image/assets/kelsey.jpg');
    project?.addLayerFromPath('Layer 2', 'C:/Users/untun/Documents/vscode/rust/image/assets/34KK-breasts.webp');
  }

  // IPC handlers for custom title bar
  ipcMain.handle('show-menu', (event, menuLabel) => {
    const menuItem = menuTemplate.find(item => item.label === menuLabel);
    if (menuItem && menuItem.submenu) {
      const submenu = Menu.buildFromTemplate(menuItem.submenu as any);
      submenu.popup({ window: BrowserWindow.fromWebContents(event.sender)! });
    }
  });

  ipcMain.handle('minimize-window', event => {
    BrowserWindow.fromWebContents(event.sender)?.minimize();
  });

  ipcMain.handle('maximize-window', event => {
    const win = BrowserWindow.fromWebContents(event.sender);
    if (win?.isMaximized()) {
      win.unmaximize();
    } else {
      win?.maximize();
    }
  });

  ipcMain.handle('close-window', event => {
    BrowserWindow.fromWebContents(event.sender)?.close();
  });

  ipcMain.handle('is-maximized', event => {
    return BrowserWindow.fromWebContents(event.sender)?.isMaximized();
  });

  if (isDev) {
    // Wait for webpack dev server to be ready
    const maxRetries = 20;
    for (let i = 0; i < maxRetries; i++) {
      try {
        await mainWindow.loadURL('http://localhost:8080');
        console.log('Dev server loaded successfully');
        break;
      } catch (err) {
        console.log(`Waiting for dev server... (${i + 1}/${maxRetries})`);
        if (i === maxRetries - 1) {
          console.error('Failed to connect to dev server:', err);
          app.quit();
          return;
        }
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
    }
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../client/index.html'));
  }
  mainWindow.on('blur', () => mainWindow.webContents.send('window-lost-focus'));
});

// Quit when all windows are closed
app.on('window-all-closed', () => {
  if (isDev) {
    // Exit with non-zero code so nodemon's exitcrash terminates the watcher
    process.exit(1);
  }
  app.quit();
});

ipcMain.handle('is-dev', () => isDev);
ipcMain.handle('toggle-dev-tools', event => {
  const win = BrowserWindow.fromWebContents(event.sender);
  if (win) {
    if (win.webContents.isDevToolsOpened()) {
      win.webContents.closeDevTools();
    } else {
      win.webContents.openDevTools();
    }
  }
});

// Events that come from the client.
import './events/clipboard';
import './events/dialogs';
import './events/drawing';
import './events/image-data';
import './events/projects';
import { openProject } from './events/projects';
import './events/tools/selection';
import './events/transform';
