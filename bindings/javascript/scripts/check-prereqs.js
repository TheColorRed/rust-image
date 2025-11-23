#!/usr/bin/env node
const { execSync } = require('child_process');

function isWindows() { return process.platform === 'win32'; }

function which(cmd) {
  try { return execSync(`where.exe ${cmd}`, { stdio: ['pipe', 'pipe', 'ignore'] }).toString().split(/\r?\n/)[0].trim(); }
  catch (e) { return null; }
}

function checkPython() {
  try {
    const python = process.env.npm_config_python || process.env.PYTHON || 'python';
    const ver = execSync(`"${python}" --version`, { stdio: ['pipe', 'pipe', 'ignore'] }).toString().trim();
    if (!/Python\s+\d+\.\d+/.test(ver)) throw new Error('Invalid Python version');
    console.log('Found Python:', python, ver);
    return true;
  } catch (e) {
    console.error('Python not detected or not executable. Please install CPython (NOT the Windows Store shim) and ensure it is on PATH.');
    return false;
  }
}

function checkCl() {
  try {
    const cl = which('cl.exe');
    if (!cl) throw new Error('cl.exe not found');
    console.log('Found MSVC cl.exe at', cl);
    return true;
  } catch (e) {
    console.error('MSVC compiler (cl.exe) not found on PATH. Please run from Developer PowerShell for Visual Studio or install the "Desktop development with C++" workload.');
    return false;
  }
}

if (isWindows()) {
  const ok = checkPython() && checkCl();
  if (!ok) process.exit(1);
}
