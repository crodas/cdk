#!/usr/bin/env node
/**
 * Fetches the prebuilt native libraries for this release and unpacks them.
 *
 * The binaries are GitHub Release assets rather than files in the repository,
 * following cdk-swift: cdk-go and cdk-dart commit theirs and carry well over
 * 100 MB per release in git forever.
 *
 * Runs as `postinstall`. A consumer installing with --ignore-scripts must run
 * it by hand; the module throws a message saying so rather than failing at
 * link time.
 */
import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import { access, mkdir, readFile, rm } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { promisify } from 'node:util';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const run = promisify(execFile);
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Each asset, and the path that proves it is already unpacked. */
const ASSETS = [
  { name: 'CashuFfi.xcframework.zip', marker: 'CashuFfi.xcframework/Info.plist', into: '.' },
  { name: 'jniLibs.zip', marker: 'android/src/main/jniLibs/arm64-v8a/libcashu_ffi.so', into: 'android/src/main' },
];

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function sha256(path) {
  const hash = createHash('sha256');
  await pipeline((await import('node:fs')).createReadStream(path), hash);
  return hash.digest('hex');
}

/** `<sha256>  <name>` per line, as `sha256sum` writes it. */
async function readChecksums() {
  const raw = await readFile(join(root, 'checksums.sha256'), 'utf8');
  return new Map(
    raw
      .split('\n')
      .filter((line) => line.trim() !== '')
      .map((line) => {
        const [sum, ...rest] = line.trim().split(/\s+/);
        return [rest.join(' '), sum];
      }),
  );
}

async function main() {
  // checksums.sha256 is written at publish time, so its absence means this is
  // a source checkout of the monorepo rather than an installed package. There
  // is no release to fetch from; `just binding-react-native-{ios,android}`
  // builds the binaries instead.
  if (!(await exists(join(root, 'checksums.sha256')))) return;

  const pkg = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));

  if ((await Promise.all(ASSETS.map((a) => exists(join(root, a.marker))))).every(Boolean)) {
    return;
  }

  const repo = (pkg.repository?.url ?? '')
    .replace(/^git\+/, '')
    .replace(/^https:\/\/github\.com\//, '')
    .replace(/\.git$/, '');
  if (repo === '') throw new Error('package.json has no repository.url to fetch binaries from');

  const checksums = await readChecksums();
  // Overridable so CI can verify freshly built assets before they are
  // published, and so a consumer can point at an internal mirror.
  const base =
    process.env.CASHU_NATIVE_BASE_URL ??
    `https://github.com/${repo}/releases/download/v${pkg.version}`;
  const tmp = join(root, '.fetch');
  await mkdir(tmp, { recursive: true });

  try {
    for (const asset of ASSETS) {
      if (await exists(join(root, asset.marker))) continue;

      const expected = checksums.get(asset.name);
      if (expected === undefined) throw new Error(`checksums.sha256 has no entry for ${asset.name}`);

      const url = `${base}/${asset.name}`;
      process.stdout.write(`@cashu/cashu-native: fetching ${asset.name}\n`);
      const response = await fetch(url);
      if (!response.ok) throw new Error(`${url} returned ${response.status} ${response.statusText}`);

      const archive = join(tmp, asset.name);
      await pipeline(Readable.fromWeb(response.body), createWriteStream(archive));

      const actual = await sha256(archive);
      if (actual !== expected) {
        throw new Error(`${asset.name} checksum mismatch\n  expected ${expected}\n  got      ${actual}`);
      }

      const into = join(root, asset.into);
      await mkdir(into, { recursive: true });
      await run('unzip', ['-q', '-o', archive, '-d', into]);
    }
  } finally {
    await rm(tmp, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(
    `\n@cashu/cashu-native: could not fetch its native libraries.\n` +
      `  ${error.message}\n\n` +
      `  If you installed with --ignore-scripts, run this by hand:\n` +
      `    node node_modules/@cashu/cashu-native/scripts/fetch-binaries.mjs\n\n`,
  );
  process.exit(1);
});
