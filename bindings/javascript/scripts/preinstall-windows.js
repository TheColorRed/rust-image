#!/usr/bin/env node
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function isWindows() {
  return process.platform === 'win32';
}

function findWindowsPython() {
  try {
    const out = execSync('where.exe python', { stdio: ['pipe', 'pipe', 'ignore'] }).toString();
    const lines = out.split(/\r?\n/).map(l => l.trim()).filter(Boolean);
    // Filter out msys/mingw/git and windows store stub
    const candidates = lines.filter(p => !/msys64|MINGW|Git\\mingw64|WindowsApps/i.test(p));
    // Try to validate candidates by executing `--version`
    for (const pth of candidates.concat(lines)) {
      try {
        const v = execSync(`"${pth}" --version`, { stdio: ['pipe', 'pipe', 'ignore'] }).toString().trim();
        if (v && /Python\s+\d+\.\d+/.test(v)) {
          return pth;
        }
      } catch (e) {
        // ignore non-executable candidates
      }
    }
    return null;
  } catch (e) {
    return null;
  }
}

function writeNpmrcWithPython(pythonPath) {
  if (!pythonPath) return;
  const npmrcPath = path.resolve(process.cwd(), '.npmrc');
  const content = `python=${pythonPath.replace(/\\/g, '\\\\')}`; // double escape for windows path safety
  // If .npmrc already contains a python config, do not override it
  try {
    if (fs.existsSync(npmrcPath)) {
      const existing = fs.readFileSync(npmrcPath, 'utf8');
      if (/^\s*python\s*=\s*/mi.test(existing)) {
        console.log('.npmrc already contains a python setting; leaving it intact.');
        return;
      }
    }
  } catch (err) {
    // ignore and proceed to write
  }
  try {
    fs.writeFileSync(npmrcPath, content + '\n', { flag: 'w' });
    console.log('Wrote .npmrc with python path to:', npmrcPath);
    console.log('python set to', pythonPath);
  } catch (err) {
    console.error('Failed to write .npmrc:', err.message);
    appendDebug('Failed to write .npmrc: ' + err.message);
  }
}

function appendDebug(msg) {
  const log = path.resolve(process.cwd(), '.preinstall-debug.log');
  try {
    const ts = new Date().toISOString();
    fs.appendFileSync(log, `[${ts}] ${msg}\n`);
  } catch (e) {
    /* ignore */
  }
}

function main() {
  if (!isWindows()) return;
  // Always write a debug header so we detect that the script ran at all
  try {
    appendDebug('preinstall start - cwd=' + process.cwd());
    appendDebug('env npm_config_python=' + (process.env.npm_config_python || '') + ' PYTHON=' + (process.env.PYTHON || ''));
    appendDebug('PATH sample: ' + (process.env.PATH || '').split(';').slice(0,5).join(';'));
  } catch (e) { /* ignore */ }
  // If user has already set npm python in env/npmrc, don't override
  const envPython = process.env.npm_config_python || process.env.PYTHON || null;
  if (envPython) {
    console.log('Using python from environment:', envPython);
    appendDebug('Using python from environment: ' + envPython);
    return;
  }

  const python = findWindowsPython();
  appendDebug('findWindowsPython result: ' + (python || 'none'));
  if (!python) {
    console.warn('Could not locate a Windows Python installation on PATH. Please install CPython from python.org and ensure it is on PATH.');
    appendDebug('No Windows python found on PATH');
    return;
  }

  if (/msys64/i.test(python)) {
    // Search for alternative that looks like a Windows install (AppData or Program Files)
    console.warn('MSYS Python detected:', python);
    appendDebug('msys python detected: ' + python);
    const lines = execSync('where.exe python', { stdio: ['pipe', 'pipe', 'ignore'] }).toString().split(/\r?\n/).map(l => l.trim()).filter(Boolean);
    const alt = lines.find(p => /\\Users\\|Program Files|AppData/i.test(p) && !/msys64/i.test(p));
    if (alt) {
      writeNpmrcWithPython(alt);
    } else {
      // Write the first non-msys if available as a fallback
      const fallback = lines.find(p => !/msys64/i.test(p)) || null;
      if (fallback) { appendDebug('Using fallback python ' + fallback); writeNpmrcWithPython(fallback); }
      else console.warn('No suitable Windows Python found on PATH. Consider installing CPython and adding it to PATH.');
    }
  } else {
    appendDebug('Using python from findWindowsPython: ' + python);
    writeNpmrcWithPython(python);
  }

  // Check for MSVC toolchain (cl.exe)
  try {
    const clOut = execSync('where.exe cl.exe', { stdio: ['pipe', 'pipe', 'ignore'] }).toString().trim();
    if (!clOut) throw new Error('not found');
  } catch (err) {
    console.warn('MSVC compiler (cl.exe) was not found on PATH. Run from a Developer PowerShell for Visual Studio or install the "Desktop development with C++" workload.');
    appendDebug('cl.exe not found');
  }

  // Check Node version for ffi-napi binary availability
  try {
    const nodeVer = process.versions.node || '';
    const major = parseInt(nodeVer.split('.')[0], 10);
    if (major >= 22) {
      console.warn('Node version ' + nodeVer + ' detected. ffi-napi may not have prebuilt binaries for very recent Node versions; expect a native build.');
    }
  } catch (e) { appendDebug('node version check failed'); }
  appendDebug('preinstall completed');
}

main();

/*
This script is intentionally conservative: it runs as part of the npm `preinstall` script and will
write a .npmrc file at the package root only when a suitable Windows CPython is detected on PATH.

It avoids changing the system configuration permanently and will not run on non-Windows platforms.
*/
