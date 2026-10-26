#!/usr/bin/env node
// Bundles plugin entries and updates file hashes; normal builds also write an installable .akzx archive.
import path from 'node:path';
import { buildPlugins } from './plugin-builder.mjs';

const args = process.argv.slice(2);
const devMode = args.includes('--dev');
const pluginDirs = args.filter(value => value !== '--dev');
await buildPlugins(pluginDirs.length ? pluginDirs : [process.cwd()], { devMode });
