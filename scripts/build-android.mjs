// Builds and deploys the Android app in one go:
//   1. builds libalakazam_mobile.so and regenerates the TypeScript bindings (packages/react-native/alakazam),
//   2. installs the app on the connected device with Gradle,
//   3. launches it.
//
// Gradle is run directly because `react-native run-android` fails to find gradlew.bat on this setup. Metro is not
// started; run `npm run start:android` in another terminal.
//
// Usage: npm run deploy:android [-- --release] [-- --no-launch]
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const isWindows = process.platform === 'win32';
const args = process.argv.slice(2);
const release = args.includes('--release');
const launch = !args.includes('--no-launch');
const appId = 'com.alakazam.mobile';

function run(p_label, p_command, p_args, p_cwd) {
  console.log(`\n==> ${p_label}`);
  const result = spawnSync(p_command, p_args, { cwd: p_cwd, stdio: 'inherit', shell: isWindows });
  if (result.status !== 0) {
    console.error(`\ndeploy-android: "${p_label}" failed (exit ${result.status ?? 'none'}).`);
    process.exit(result.status ?? 1);
  }
}

run(
  'Build Rust libraries and bindings',
  'npm',
  ['run', release ? 'build:android:release' : 'build:android', '-w', 'packages/react-native/alakazam'],
  root,
);

const androidDir = path.join(root, 'apps', 'mobile', 'android');
const gradlew = isWindows ? `"${path.join(androidDir, 'gradlew.bat')}"` : path.join(androidDir, 'gradlew');
run(
  'Install the app with Gradle',
  gradlew,
  [release ? 'app:installRelease' : 'app:installDebug', '-PreactNativeDevServerPort=8081'],
  androidDir,
);

if (launch) {
  run('Launch the app', 'adb', ['shell', 'monkey', '-p', appId, '-c', 'android.intent.category.LAUNCHER', '1'], root);
}
console.log('\ndeploy-android: done.');
