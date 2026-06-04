import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import assert from 'node:assert/strict';
import { assembleLatestJson } from './assemble-tauri-latest-json.mjs';

function makeAssets(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cc-switch-latest-json-'));
  for (const [name, content] of Object.entries(files)) {
    fs.writeFileSync(path.join(dir, name), content);
  }
  return dir;
}

test('assembles signed updater metadata for all supported platforms', () => {
  const dir = makeAssets({
    'CC-Switch-Remote-v3.16.2-macOS.tar.gz': 'mac',
    'CC-Switch-Remote-v3.16.2-macOS.tar.gz.sig': 'mac-sig',
    'CC-Switch-Remote-v3.16.2-Linux-x86_64.AppImage': 'linux-x64',
    'CC-Switch-Remote-v3.16.2-Linux-x86_64.AppImage.sig': 'linux-x64-sig',
    'CC-Switch-Remote-v3.16.2-Linux-arm64.AppImage': 'linux-arm64',
    'CC-Switch-Remote-v3.16.2-Linux-arm64.AppImage.sig': 'linux-arm64-sig',
    'CC-Switch-Remote-v3.16.2-Windows.msi': 'windows',
    'CC-Switch-Remote-v3.16.2-Windows.msi.sig': 'windows-sig',
  });

  const latest = assembleLatestJson({
    assetsDir: dir,
    repo: 'xiaoY233/cc-switch-remote',
    tag: 'v3.16.2',
    pubDate: '2026-06-04T00:00:00Z',
  });

  assert.equal(latest.version, '3.16.2');
  assert.equal(latest.platforms['darwin-aarch64'].signature, 'mac-sig');
  assert.equal(latest.platforms['darwin-x86_64'].signature, 'mac-sig');
  assert.equal(latest.platforms['linux-x86_64'].signature, 'linux-x64-sig');
  assert.equal(latest.platforms['linux-aarch64'].signature, 'linux-arm64-sig');
  assert.equal(latest.platforms['windows-x86_64'].signature, 'windows-sig');
});

test('fails when an updater artifact is missing its signature', () => {
  const dir = makeAssets({
    'CC-Switch-Remote-v3.16.2-Windows.msi': 'windows',
  });

  assert.throws(
    () => assembleLatestJson({
      assetsDir: dir,
      repo: 'xiaoY233/cc-switch-remote',
      tag: 'v3.16.2',
      requiredPlatforms: ['windows-x86_64'],
    }),
    /missing \.sig files.*Windows\.msi/,
  );
});

test('uses an asset names file so metadata repair does not download large artifacts', () => {
  const dir = makeAssets({
    'CC-Switch-Remote-v3.16.2-macOS.tar.gz.sig': 'mac-sig',
    'CC-Switch-Remote-v3.16.2-Linux-x86_64.AppImage.sig': 'linux-x64-sig',
    'CC-Switch-Remote-v3.16.2-Linux-arm64.AppImage.sig': 'linux-arm64-sig',
    'CC-Switch-Remote-v3.16.2-Windows.msi.sig': 'windows-sig',
  });
  const assetNamesFile = path.join(dir, 'asset-names.txt');
  fs.writeFileSync(assetNamesFile, [
    'CC-Switch-Remote-v3.16.2-macOS.tar.gz',
    'CC-Switch-Remote-v3.16.2-macOS.tar.gz.sig',
    'CC-Switch-Remote-v3.16.2-Linux-x86_64.AppImage',
    'CC-Switch-Remote-v3.16.2-Linux-x86_64.AppImage.sig',
    'CC-Switch-Remote-v3.16.2-Linux-arm64.AppImage',
    'CC-Switch-Remote-v3.16.2-Linux-arm64.AppImage.sig',
    'CC-Switch-Remote-v3.16.2-Windows.msi',
    'CC-Switch-Remote-v3.16.2-Windows.msi.sig',
  ].join('\n'));

  const latest = assembleLatestJson({
    assetsDir: dir,
    assetNamesFile,
    repo: 'xiaoY233/cc-switch-remote',
    tag: 'v3.16.2',
  });

  assert.equal(latest.platforms['windows-x86_64'].signature, 'windows-sig');
  assert.equal(latest.platforms['linux-aarch64'].signature, 'linux-arm64-sig');
});
