// Shuts down everything the Android workflow leaves running, so folders are unlocked and ports are free:
//   1. `npm run watch:android` and `npm run deploy:android`, first, so they cannot start anything again,
//   2. Metro (whatever listens on port 8081),
//   3. the Gradle daemons,
//   4. the adb port forwards on every connected device.
//
// Usage: npm run stop:android
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const isWindows = process.platform === 'win32';
const androidDir = path.join(root, 'apps', 'alakazam', 'mobile', 'android');
const scriptPattern = /scripts[\\/](watch|build)-android\.mjs/;

/** Stops a process and everything it started (npm, cargo, Gradle), not only the top process. */
function kill(p_pid) {
  if (isWindows) spawnSync('taskkill', ['/pid', String(p_pid), '/T', '/F'], { stdio: 'ignore' });
  else spawnSync('kill', ['-TERM', String(p_pid)], { stdio: 'ignore' });
}

function output(p_command, p_args) {
  const result = spawnSync(p_command, p_args, { encoding: 'utf8', shell: false });
  return result.error ? '' : result.stdout;
}

// 1. The watcher and the deploy script.
const scriptPids = isWindows
  ? output('powershell', ['-NoProfile', '-Command', "Get-CimInstance Win32_Process -Filter \"Name='node.exe'\" | ForEach-Object { \"$($_.ProcessId)`t$($_.CommandLine)\" }"])
      .split(/\r?\n/)
      .map((line) => line.split('\t'))
      .filter(([pid, commandLine]) => Number(pid) !== process.pid && scriptPattern.test(commandLine ?? ''))
      .map(([pid]) => Number(pid))
  : output('pgrep', ['-f', scriptPattern.source.replace('[\\\\/]', '/')])
      .split(/\s+/)
      .filter(Boolean)
      .map(Number)
      .filter((pid) => pid !== process.pid);
for (const pid of scriptPids) kill(pid);
console.log(scriptPids.length ? `Stopped ${scriptPids.length} watch/deploy process(es).` : 'No watch/deploy script running.');

// 2. Metro.
const metroPids = new Set(
  isWindows
    ? output('netstat', ['-ano', '-p', 'tcp'])
        .split(/\r?\n/)
        .map((line) => line.trim().split(/\s+/))
        .filter((columns) => columns[3] === 'LISTENING' && columns[1]?.endsWith(':8081'))
        .map((columns) => Number(columns[4]))
    : output('lsof', ['-ti', 'tcp:8081', '-sTCP:LISTEN'])
        .split(/\s+/)
        .filter(Boolean)
        .map(Number),
);
for (const pid of metroPids) kill(pid);
console.log(metroPids.size ? 'Stopped Metro.' : 'Metro was not running.');

// 3. Gradle daemons.
const gradlew = path.join(androidDir, isWindows ? 'gradlew.bat' : 'gradlew');
const gradle = spawnSync(`"${gradlew}" --stop`, { cwd: androidDir, encoding: 'utf8', shell: true });
console.log(gradle.status === 0 ? gradle.stdout.trim().split(/\r?\n/).pop() : 'Could not stop the Gradle daemon (is the Android project present?).');

// 4. adb port forwards, on every device (a plain `adb reverse` fails when more than one is attached).
const devices = output('adb', ['devices'])
  .split(/\r?\n/)
  .slice(1)
  .map((line) => line.split(/\s+/))
  .filter(([, state]) => state === 'device')
  .map(([serial]) => serial);
for (const serial of devices) spawnSync('adb', ['-s', serial, 'reverse', '--remove-all'], { stdio: 'ignore' });
console.log(devices.length ? `Removed port forwards on ${devices.length} device(s).` : 'No adb devices connected.');
