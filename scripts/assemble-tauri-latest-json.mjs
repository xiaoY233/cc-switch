#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_REQUIRED_PLATFORMS = [
  'darwin-aarch64',
  'darwin-x86_64',
  'linux-x86_64',
  'linux-aarch64',
  'windows-x86_64',
];

function parseArgs(argv) {
  const args = {
    assetsDir: 'dl',
    output: 'latest.json',
    requiredPlatforms: DEFAULT_REQUIRED_PLATFORMS,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const next = () => {
      i += 1;
      if (i >= argv.length) {
        throw new Error(`Missing value for ${arg}`);
      }
      return argv[i];
    };

    if (arg === '--assets-dir') args.assetsDir = next();
    else if (arg === '--repo') args.repo = next();
    else if (arg === '--tag') args.tag = next();
    else if (arg === '--output') args.output = next();
    else if (arg === '--asset-names-file') args.assetNamesFile = next();
    else if (arg === '--pub-date') args.pubDate = next();
    else if (arg === '--notes') args.notes = next();
    else if (arg === '--required-platforms') {
      const value = next();
      args.requiredPlatforms = value === 'none'
        ? []
        : value.split(',').map((item) => item.trim()).filter(Boolean);
    } else if (arg === '--help' || arg === '-h') {
      args.help = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return args;
}

function usage() {
  return `Usage: node scripts/assemble-tauri-latest-json.mjs --repo owner/repo --tag v1.2.3 [options]

Options:
  --assets-dir <dir>              Release assets directory, default: dl
  --asset-names-file <file>       Optional release asset names, JSON or newline text
  --output <file>                 Output file, default: latest.json
  --pub-date <iso-date>           Publication date, default: current UTC time
  --notes <text>                  Release notes, default: Release <tag>
  --required-platforms <list>     Comma-separated platform keys, or "none"
`;
}

function readAssetNamesFile(assetNamesFile) {
  if (!assetNamesFile) return [];

  const text = fs.readFileSync(assetNamesFile, 'utf8');
  try {
    const parsed = JSON.parse(text);
    if (Array.isArray(parsed)) {
      return parsed.map((item) => typeof item === 'string' ? item : item?.name).filter(Boolean);
    }
    if (Array.isArray(parsed?.assets)) {
      return parsed.assets.map((item) => typeof item === 'string' ? item : item?.name).filter(Boolean);
    }
  } catch (_) {
    // Fall through to newline text format.
  }

  return text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
}

function readAssetMap(assetsDir, assetNamesFile) {
  const localEntries = fs.existsSync(assetsDir)
    ? fs.readdirSync(assetsDir, { withFileTypes: true })
      .filter((entry) => entry.isFile())
      .map((entry) => entry.name)
    : [];
  const entries = [...new Set([...localEntries, ...readAssetNamesFile(assetNamesFile)])];
  const signatures = new Map();

  for (const name of localEntries) {
    if (!name.endsWith('.sig')) continue;
    const assetName = name.slice(0, -4);
    signatures.set(assetName, fs.readFileSync(path.join(assetsDir, name), 'utf8').trim());
  }

  return { entries, signatures };
}

function classifyUpdaterArtifact(name) {
  if (name.endsWith('.tar.gz')) return ['darwin-aarch64', 'darwin-x86_64'];
  if (/-Linux-x86_64\.AppImage$/i.test(name)) return ['linux-x86_64'];
  if (/-Linux-arm64\.AppImage$/i.test(name)) return ['linux-aarch64'];
  if (/-Windows\.msi$/i.test(name)) return ['windows-x86_64'];
  return [];
}

export function assembleLatestJson({
  assetsDir,
  repo,
  tag,
  assetNamesFile,
  pubDate = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  notes = `Release ${tag}`,
  requiredPlatforms = DEFAULT_REQUIRED_PLATFORMS,
}) {
  if (!repo) throw new Error('--repo is required');
  if (!tag) throw new Error('--tag is required');

  const version = tag.replace(/^v/, '');
  const baseUrl = `https://github.com/${repo}/releases/download/${tag}`;
  const { entries, signatures } = readAssetMap(assetsDir, assetNamesFile);
  const platforms = {};
  const updaterArtifacts = [];
  const unsignedUpdaterArtifacts = [];

  for (const name of entries) {
    if (name.endsWith('.sig')) continue;
    const platformKeys = classifyUpdaterArtifact(name);
    if (platformKeys.length === 0) continue;

    updaterArtifacts.push(name);
    const signature = signatures.get(name);
    if (!signature) {
      unsignedUpdaterArtifacts.push(name);
      continue;
    }

    for (const platformKey of platformKeys) {
      platforms[platformKey] = {
        signature,
        url: `${baseUrl}/${name}`,
      };
    }
  }

  const missingPlatforms = requiredPlatforms.filter((platform) => !platforms[platform]);
  const errors = [];
  if (unsignedUpdaterArtifacts.length > 0) {
    errors.push(`Updater artifacts are missing .sig files: ${unsignedUpdaterArtifacts.join(', ')}`);
  }
  if (missingPlatforms.length > 0) {
    errors.push(`latest.json is missing required platforms: ${missingPlatforms.join(', ')}`);
  }
  if (updaterArtifacts.length === 0) {
    errors.push('No updater artifacts found (.tar.gz, Linux AppImage, Windows MSI)');
  }
  if (errors.length > 0) {
    throw new Error(errors.join('\n'));
  }

  return {
    version,
    notes,
    pub_date: pubDate,
    platforms,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(usage());
    return;
  }

  const latestJson = assembleLatestJson({
    assetsDir: args.assetsDir,
    repo: args.repo,
    tag: args.tag,
    assetNamesFile: args.assetNamesFile,
    pubDate: args.pubDate,
    notes: args.notes,
    requiredPlatforms: args.requiredPlatforms,
  });

  fs.writeFileSync(args.output, `${JSON.stringify(latestJson, null, 2)}\n`);
  console.log(`Generated ${args.output} with platforms: ${Object.keys(latestJson.platforms).join(', ')}`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
