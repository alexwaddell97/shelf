// Builds a signed Windows release and the `latest.json` that installed copies of Shelf check
// for updates.
//
//   npm run release                          build, then print the steps to publish
//   npm run release -- --notes-file notes.md include release notes ("What's new")
//   npm run release -- --url-base <url>      point the download somewhere else (for testing)
//
// Signing needs the private updater key, kept outside the repo. By default it's read from
// ~/.tauri/shelf-updater.key; set TAURI_SIGNING_PRIVATE_KEY to use another. Keep a backup:
// without it, copies already out there can't be sent updates.

import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const repo = 'alexwaddell97/shelf';

function option(name) {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 ? process.argv[i + 1] : undefined;
}

const { version } = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const tag = `v${version}`;
const notesFile = option('notes-file');
const notes = notesFile ? readFileSync(notesFile, 'utf8').trim() : '';
const urlBase = option('url-base') ?? `https://github.com/${repo}/releases/download/${tag}`;

const key = process.env.TAURI_SIGNING_PRIVATE_KEY ?? join(homedir(), '.tauri', 'shelf-updater.key');
if (!process.env.TAURI_SIGNING_PRIVATE_KEY && !existsSync(key)) {
  console.error(`No updater key at ${key}. Set TAURI_SIGNING_PRIVATE_KEY to its path.`);
  process.exit(1);
}

console.log(`Building Shelf ${version}…`);
const build = spawnSync(process.execPath, [join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build'], {
  cwd: root,
  stdio: 'inherit',
  env: {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: key,
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? ''
  }
});
if (build.status !== 0) process.exit(build.status ?? 1);

const installer = `Shelf_${version}_x64-setup.exe`;
const built = join(root, 'src-tauri/target/release/bundle/nsis', installer);
if (!existsSync(`${built}.sig`)) {
  console.error(`The build made no signature (${installer}.sig). Is createUpdaterArtifacts on?`);
  process.exit(1);
}

const out = join(root, 'release', tag);
mkdirSync(out, { recursive: true });
copyFileSync(built, join(out, installer));
const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: readFileSync(`${built}.sig`, 'utf8').trim(),
      url: `${urlBase}/${installer}`
    }
  }
};
writeFileSync(join(out, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');

const rel = (p) => p.replace(root + '\\', '').replaceAll('\\', '/');
console.log(`
Ready in ${rel(out)}:
  ${installer}
  latest.json

To publish (GitHub first: installed copies look for latest.json on the latest release):
  gh release create ${tag} "${rel(join(out, installer))}" "${rel(join(out, 'latest.json'))}" --title "Shelf ${version}"${notesFile ? ` --notes-file "${notesFile}"` : ' --notes ""'}
  butler push "${rel(join(out, installer))}" <your-itch-user>/shelf:windows --userversion ${version}
`);
