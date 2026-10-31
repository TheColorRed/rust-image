#!/usr/bin/env node

const [command, ...args] = process.argv.slice(2);

const scripts = {
  build: './build-plugins.mjs',
  dev: './dev-server.mjs',
};

if (!scripts[command]) {
  console.error('Usage: npx @alakazam/sdk <dev|build> [plugin-directory ...]');
  process.exitCode = 1;
} else {
  process.argv.splice(2, 1);
  await import(scripts[command]);
}
