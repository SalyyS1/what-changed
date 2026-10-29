import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { prepareThirdPartyReview } from '../prepare-third-party-review.mjs';
import { CARGO_EDGES, TARGET, sha256 } from './filesystem.mjs';

export function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'what-changed-review-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (file, bytes) => {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), bytes);
  };
  write('src-tauri/Cargo.lock', 'locked cargo fixture\r\n');
  write('src-tauri/Cargo.toml', '[package]\nname="what-changed"\n');
  write('bun.lock', 'locked bun fixture\n');
  write('scripts/third-party-notices-manifest.json', '{"status":"pending","entries":[]}\n');
  const inventory = { schemaVersion: 2, packages: [
    { name: '@example/browser', version: '4.5.6' },
    { name: 'declaration-only', version: '2.0.0' },
    { name: 'unexpressed', version: '1.0.0' },
  ], cssPackages: [], unresolvedNodeModules: 0, unresolvedCssImports: 0 };
  const metadata = { version: 1, packages: [
    { name: 'what-changed', version: '0.1.0', manifest_path: join(root, 'src-tauri/Cargo.toml') },
    ...['crate-one', 'libsqlite3-sys', 'proc-only'].map((name) => ({
      name, version: '1.2.3', license: 'MIT OR Apache-2.0',
      source: 'registry+https://github.com/rust-lang/crates.io-index',
      manifest_path: join(root, `.cargo/registry/src/index/${name}-1.2.3/Cargo.toml`),
    })),
  ] };
  for (const item of metadata.packages.slice(1)) {
    const prefix = `.cargo/registry/src/index/${item.name}-1.2.3`;
    write(`${prefix}/Cargo.toml`, `[package]\nname="${item.name}"\n`);
    write(`${prefix}/Cargo.toml.orig`, '[package]\n# original manifest\n');
    write(`${prefix}/.cargo_vcs_info.json`, '{"git":{"sha1":"fixture"}}\n');
    write(`${prefix}/LICENSE_MIT`, 'Copyright fixture\r\nMIT original bytes\r\n');
  }
  write('.cargo/registry/src/index/crate-one-1.2.3/NOTICE', 'NOTICE original bytes\r\n');
  write('.cargo/registry/src/index/libsqlite3-sys-1.2.3/sqlite3/sqlite3.c',
    '#define SQLITE_VERSION "3.53.2"\n#define SQLITE_SOURCE_ID "fixture-checkin"\n');
  for (const item of inventory.packages) {
    write(`node_modules/${item.name}/package.json`, JSON.stringify({ ...item,
      ...(item.name === 'unexpressed' ? {} : { license: 'MIT OR Apache-2.0' }),
    }));
  }
  write('node_modules/@example/browser/LICENSE_MIT', Buffer.from([0xef, 0xbb, 0xbf, ...Buffer.from('MIT untouched\r\n')]));
  write('node_modules/@example/browser/LICENSE_APACHE-2.0', 'Apache untouched\n');
  write('node_modules/declaration-only/LICENSE.spdx', 'PackageLicenseDeclared: MIT\r\n');
  write('node_modules/unexpressed/LICENSE', 'MIT original\n');
  const cargoTree = join(root, 'cargo-tree.txt');
  const cargoMetadata = join(root, 'cargo-metadata.json');
  const inputSnapshot = join(root, 'input-snapshot.json');
  write('cargo-tree.txt', 'what-changed v0.1.0 (local)\ncrate-one v1.2.3\nlibsqlite3-sys v1.2.3\ncrate-one v1.2.3 (*)\n');
  const saveInputs = () => {
    write('dist-installer/frontend-inputs.json', JSON.stringify(inventory));
    write('cargo-metadata.json', JSON.stringify(metadata));
    const inputs = { schemaVersion: 1, target: TARGET, cargoEdges: CARGO_EDGES };
    for (const [key, file] of Object.entries({ cargoLockSha256: 'src-tauri/Cargo.lock', bunLockSha256: 'bun.lock',
      cargoManifestSha256: 'src-tauri/Cargo.toml', frontendInputsSha256: 'dist-installer/frontend-inputs.json',
      cargoTreeSha256: 'cargo-tree.txt', cargoMetadataSha256: 'cargo-metadata.json' })) {
      inputs[key] = sha256(readFileSync(join(root, file)));
    }
    write('input-snapshot.json', JSON.stringify(inputs));
  };
  saveInputs();
  const options = { root, cargoTree, cargoMetadata, inputSnapshot, cargoSourceRoots: [join(root, '.cargo/registry/src')] };
  return { root, write, inventory, metadata, options, saveInputs, run: () => prepareThirdPartyReview(options) };
}
