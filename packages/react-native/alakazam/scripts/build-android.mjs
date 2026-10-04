// Builds libalakazam_mobile.so for Android and regenerates the TypeScript
// bindings via `ubrn build jsi2 android --and-generate`.
//
// abra's JPEG support vendors libjpeg-turbo, which the `cmake` crate builds.
// Out of the box that fails when cross-compiling (no NDK toolchain, no Ninja),
// so this points cmake at the NDK's toolchain file and the SDK's cmake/ninja
// through target-scoped env vars that leave desktop builds untouched.
//
// Extra arguments are passed through to ubrn, e.g. `--release` or
// `--targets arm64-v8a`.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const isWindows = process.platform === 'win32';
const exe = (name) => (isWindows ? `${name}.exe` : name);

const TRIPLES = ['aarch64-linux-android', 'x86_64-linux-android', 'armv7-linux-androideabi', 'i686-linux-android'];

function fail(message) {
  console.error(`build-android: ${message}`);
  process.exit(1);
}

function findSdk() {
  const candidates = [
    process.env.ANDROID_HOME,
    process.env.ANDROID_SDK_ROOT,
    isWindows && process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Android', 'Sdk'),
    process.platform === 'darwin' && path.join(os.homedir(), 'Library', 'Android', 'sdk'),
    path.join(os.homedir(), 'Android', 'Sdk'),
  ];
  const sdk = candidates.find((dir) => dir && existsSync(dir));
  if (!sdk) fail('Android SDK not found; set ANDROID_HOME.');
  return sdk;
}

/** Returns the newest version directory under `p_dir`, or undefined. */
function newestVersionDir(p_dir) {
  if (!existsSync(p_dir)) return undefined;
  const versions = readdirSync(p_dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && /^\d/.test(entry.name))
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
  return versions.length ? path.join(p_dir, versions.at(-1)) : undefined;
}

const sdk = findSdk();
const ndk = process.env.ANDROID_NDK_HOME ?? newestVersionDir(path.join(sdk, 'ndk'));
if (!ndk || !existsSync(ndk)) fail(`no NDK found under ${path.join(sdk, 'ndk')}; install one with the SDK Manager.`);

const cmakeDir = process.env.ANDROID_CMAKE_HOME ?? newestVersionDir(path.join(sdk, 'cmake'));
const cmakeBin = cmakeDir && path.join(cmakeDir, 'bin');
if (!cmakeBin || !existsSync(path.join(cmakeBin, exe('ninja')))) {
  fail(`no SDK cmake with ninja found under ${path.join(sdk, 'cmake')}; install "CMake" with the SDK Manager.`);
}

const toolchainFile = path.join(ndk, 'build', 'cmake', 'android.toolchain.cmake');
const env = {
  ...process.env,
  ANDROID_HOME: sdk,
  ANDROID_NDK_HOME: ndk,
  PATH: `${cmakeBin}${path.delimiter}${process.env.PATH}`,
};
for (const triple of TRIPLES) {
  const suffix = triple.replaceAll('-', '_');
  env[`CMAKE_TOOLCHAIN_FILE_${suffix}`] = toolchainFile;
  env[`CMAKE_GENERATOR_${suffix}`] = 'Ninja';
  env[`CMAKE_${suffix}`] = path.join(cmakeBin, exe('cmake'));
}

const ubrnCli = path.join(path.dirname(require.resolve('uniffi-bindgen-react-native/package.json')), 'bin', 'cli.cjs');
const args = [ubrnCli, 'build', 'jsi2', 'android', '--and-generate', '--config', 'ubrn.config.yaml', ...process.argv.slice(2)];

console.log(`build-android: NDK ${path.basename(ndk)}, cmake ${path.basename(cmakeDir)}`);
const result = spawnSync(process.execPath, args, { cwd: packageRoot, env, stdio: 'inherit' });
process.exit(result.status ?? 1);
