// Builds and deploys the Android app in one go:
//   1. finds every connected device (or the one named with --device),
//   2. builds libalakazam_mobile.so and regenerates the TypeScript bindings (packages/react-native/alakazam),
//   3. installs the app on each device with Gradle,
//   4. forwards Metro to each device and launches the app.
//
// Debug builds load their JavaScript from Metro, so it is started if nothing answers on port 8081: in its own terminal
// window on Windows, in the background (output in metro.log) elsewhere. It starts first so it warms up while Rust
// compiles, and it keeps running after the deploy ends.
// Release builds bundle their JavaScript and skip Metro.
//
// Gradle is run directly because `react-native run-android` fails to find gradlew.bat on this setup.
//
// --metro-only skips the build, install and launch: it only starts Metro and forwards it to the devices.
//
// --gradle-only skips the Rust build and Metro and only runs the Gradle install (debug, or release with --release).
//
// Usage: npm run dev:android [-- --release] [-- --no-launch] [-- --no-metro] [-- --metro-only] [-- --gradle-only] [-- --device <serial>]
import { spawn, spawnSync } from 'node:child_process';
import { closeSync, existsSync, openSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const isWindows = process.platform === 'win32';
const args = process.argv.slice(2);
const release = args.includes('--release');
const metroOnly = args.includes('--metro-only');
const launch = !args.includes('--no-launch') && !metroOnly;
const gradleOnly = args.includes('--gradle-only');
const startMetro = !release && !gradleOnly && !args.includes('--no-metro');
const deviceArgIndex = args.findIndex((arg) => arg === '--device' || arg.startsWith('--device='));
const requestedDevice = deviceArgIndex < 0 ? undefined : args[deviceArgIndex] === '--device' ? args[deviceArgIndex + 1] : args[deviceArgIndex].slice('--device='.length);
const ownFlags = new Set(['--release', '--no-launch', '--no-metro', '--metro-only', '--gradle-only']);
const forwardedArgs = args.filter((arg, index) => !ownFlags.has(arg) && !arg.startsWith('--device') && args[index - 1] !== '--device');
const appId = release ? 'com.alakazam.mobile' : 'com.alakazam.mobile.debug';
const metroUrl = 'http://127.0.0.1:8081/status';
const startedAt = Date.now();

// Android Studio sets one of these SDK variables, but does not necessarily add
// platform-tools to PATH. Resolve adb here so the debug-only Metro reverse
// tunnel works from a regular terminal as well.
function resolveAdb() {
  const executable = isWindows ? 'adb.exe' : 'adb';
  for (const sdk of [process.env.ANDROID_HOME, process.env.ANDROID_SDK_ROOT]) {
    if (!sdk) continue;
    const candidate = path.join(sdk, 'platform-tools', executable);
    if (existsSync(candidate)) return candidate;
  }
  return executable;
}

const adbCommand = resolveAdb();

function fail(p_message) {
  console.error(`\ndeploy-android: ${p_message}`);
  process.exit(1);
}

if (deviceArgIndex >= 0 && !requestedDevice) fail('--device requires an ADB serial.');

function run(p_label, p_command, p_args, p_cwd, p_env) {
  console.log(`\n==> ${p_label}`);
  const stepStartedAt = Date.now();
  const result = spawnSync(p_command, p_args, { cwd: p_cwd, stdio: 'inherit', shell: isWindows, env: { ...process.env, ...p_env } });
  if (result.status !== 0) fail(`"${p_label}" failed (exit ${result.status ?? 'none'}).`);
  console.log(`    ${((Date.now() - stepStartedAt) / 1000).toFixed(1)}s`);
}

function adb(p_args) {
  const result = spawnSync(adbCommand, p_args, { encoding: 'utf8', shell: false });
  if (result.error) fail(`could not run adb (${result.error.message}). Set ANDROID_HOME or add Android platform-tools to PATH.`);
  return result.stdout.trim();
}

const metroIsRunning = () =>
  fetch(metroUrl, { signal: AbortSignal.timeout(1000) }).then(
    (response) => response.text().then((text) => text === 'packager-status:running'),
    () => false,
  );

// 1. Pick the devices. --device remains useful when only one phone or tablet
// should receive a build; otherwise deploy to every authorized connection.
const attached = adb(['devices'])
  .split(/\r?\n/)
  .slice(1)
  .map((line) => line.split(/\s+/))
  .filter(([, state]) => state === 'device')
  .map(([serial]) => serial);
if (attached.length === 0) fail('no device is connected (or it is unauthorized/offline). Check `adb devices`.');
if (requestedDevice && !attached.includes(requestedDevice)) fail(`device "${requestedDevice}" is not connected. Connected:\n  ${attached.join('\n  ')}`);
const devices = requestedDevice ? [requestedDevice] : attached;
console.log(`deploy-android: ${release ? 'release' : 'debug'} build for ${devices.join(', ')}`);

// 2. Start Metro (debug only) so it warms up while Rust compiles.
let metroWasRunning = true;
if (startMetro) {
  metroWasRunning = await metroIsRunning();
  if (metroWasRunning) {
    console.log('\n==> Metro is already running');
  } else {
    if (isWindows) {
      console.log('\n==> Start Metro in a new terminal window');
      spawn('cmd', ['/c', 'start', '"Metro"', 'cmd', '/k', 'npm run start:android'], { cwd: root, stdio: 'ignore', detached: true, windowsVerbatimArguments: true }).unref();
    } else {
      console.log('\n==> Start Metro in the background (output: metro.log)');
      const log = openSync(path.join(root, 'metro.log'), 'w');
      spawn('npm', ['run', 'start:android'], { cwd: root, stdio: ['ignore', log, log], detached: true }).unref();
      closeSync(log);
    }
  }
}

if (!metroOnly) {
  // 3. Rust libraries and TypeScript bindings (skipped with --gradle-only, for a change in Kotlin or the manifest).
  if (!gradleOnly) {
    run(
      'Build Rust libraries and bindings',
      'npm',
      ['run', release ? 'build:android:release' : 'build:android', '-w', 'packages/react-native/alakazam', '--', ...(forwardedArgs.some((arg) => arg.startsWith('--targets')) ? [] : ['--targets', 'arm64-v8a']), ...forwardedArgs],
      root,
    );
  }

  // 4. Install with Gradle. Reusing the Gradle build means later device installs
  // are incremental, while ANDROID_SERIAL keeps each install unambiguous.
  const androidDir = path.join(root, 'apps', 'alakazam', 'mobile', 'android');
  const gradlew = isWindows ? `"${path.join(androidDir, 'gradlew.bat')}"` : path.join(androidDir, 'gradlew');
  for (const device of devices) {
    run(`Install the app with Gradle (${device})`, gradlew, [`app:install${release ? 'Release' : 'Debug'}`, '-PreactNativeDevServerPort=8081'], androidDir, { ANDROID_SERIAL: device });
  }
}

// 5. Wait for Metro, forward it to the device and launch.
if (startMetro) {
  if (!metroWasRunning) {
    console.log('\n==> Wait for Metro');
    let ready = false;
    for (let attempt = 0; attempt < 60 && !ready; attempt++) {
      ready = await metroIsRunning();
      if (!ready) await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    if (!ready) fail('Metro did not start within 60 seconds. Check the Metro terminal window (or metro.log).');
  }
  for (const device of devices) {
    run(`Forward Metro to ${device}`, adbCommand, ['-s', device, 'reverse', 'tcp:8081', 'tcp:8081'], root);
  }
}

if (launch) {
  // Installing over a running app leaves its old process on screen (black, with the old native library). Stop it so the launch starts fresh.
  for (const device of devices) {
    run(`Stop the running app (${device})`, adbCommand, ['-s', device, 'shell', 'am', 'force-stop', appId], root);
    run(`Launch the app (${device})`, adbCommand, ['-s', device, 'shell', 'monkey', '-p', appId, '-c', 'android.intent.category.LAUNCHER', '1'], root);
  }
}
console.log(`\ndeploy-android: done in ${((Date.now() - startedAt) / 1000).toFixed(0)}s.`);
