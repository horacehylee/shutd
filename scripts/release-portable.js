// Builds the portable Windows exe and publishes it as a GitHub release.
// Tag convention: v<version>. Bump the version in package.json,
// src-tauri/tauri.conf.json, and src-tauri/Cargo.toml before releasing so
// the tag is unique. The uploaded asset keeps the plain `shutd.exe` name.

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';

function parseArguments(argv) {
  const options = { changes: [], dryRun: false };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--dry-run') {
      options.dryRun = true;
    } else if (argv[index] === '--changes') {
      const change = argv[index + 1]?.trim();
      if (!change) {
        console.error('--changes requires a release change description.');
        process.exit(1);
      }
      options.changes.push(change);
      index += 1;
    } else {
      console.error(`Unknown argument: ${argv[index]}`);
      console.error(
        'Usage: node scripts/release-portable.js --changes "..." [--changes "..."] [--dry-run]',
      );
      process.exit(1);
    }
  }

  if (options.changes.length === 0) {
    console.error('Provide at least one GitHub release change with --changes "..."');
    process.exit(1);
  }
  return options;
}

function latestTag() {
  const result = spawnSync('git', ['describe', '--tags', '--abbrev=0'], { encoding: 'utf8' });
  return result.status === 0 ? result.stdout.trim() : null;
}

// All three version locations must match before a release; a partial bump
// would otherwise tag one version while shipping another.
function assertVersionsMatch() {
  const packageVersion = JSON.parse(readFileSync('package.json', 'utf8')).version;
  const tauriVersion = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
  const cargoVersion = readFileSync('src-tauri/Cargo.toml', 'utf8').match(
    /^version = "(.+)"$/m,
  )?.[1];
  if (packageVersion !== tauriVersion || packageVersion !== cargoVersion) {
    console.error(
      `Version mismatch: package.json=${packageVersion} tauri.conf.json=${tauriVersion} Cargo.toml=${cargoVersion}. Bump all three.`,
    );
    process.exit(1);
  }
  return packageVersion;
}

function showChangeSummary(previousTag) {
  if (!previousTag) {
    console.log('No previous tag found; skipping the change summary.');
    return;
  }

  console.log(`Changes since ${previousTag}:`);
  run('git', ['log', '--oneline', '--no-decorate', `${previousTag}..HEAD`]);
  run('git', ['diff', '--stat', `${previousTag}..HEAD`]);
}

function releaseNotes(changes) {
  return [
    "## What's Changed",
    '',
    ...changes.map((change) => `- ${change}`),
  ].join('\n');
}

const { changes, dryRun } = parseArguments(process.argv.slice(2));

function run(command, args, useShell = false) {
  const result = spawnSync(command, args, { stdio: 'inherit', shell: useShell });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function findGh() {
  const candidates = [
    'gh',
    'C:\\Program Files\\GitHub CLI\\gh.exe',
    'C:\\ProgramData\\chocolatey\\bin\\gh.exe',
  ];
  for (const candidate of candidates) {
    if (candidate === 'gh' || existsSync(candidate)) {
      const probe = spawnSync(candidate, ['--version'], { stdio: 'ignore' });
      if (probe.status === 0) {
        return candidate;
      }
    }
  }
  console.error(
    'GitHub CLI (gh) not found. Install it with `choco install gh` and run `gh auth login`.',
  );
  process.exit(1);
}

function githubRepository() {
  const result = spawnSync('git', ['remote', 'get-url', 'origin'], { encoding: 'utf8' });
  const remote = result.stdout?.trim();
  const match = remote?.match(/github\.com[/:](.+?)\/(.+?)(?:\.git)?$/i);
  if (!match) {
    console.error(`Could not resolve a GitHub repository from origin: ${remote ?? 'not found'}`);
    process.exit(1);
  }
  return `${match[1]}/${match[2]}`;
}

const version = assertVersionsMatch();
const tag = `v${version}`;
const exePath = 'src-tauri/target/release/shutd.exe';
const notes = releaseNotes(changes);

showChangeSummary(latestTag());

console.log(`Building portable exe for ${tag}...`);
// corepack/pnpm are .cmd shims on Windows, which spawnSync cannot exec
// directly; the build arguments contain no spaces, so the shell is safe here.
run('corepack', ['pnpm', 'build:portable'], process.platform === 'win32');

if (!existsSync(exePath)) {
  console.error(`Build finished but ${exePath} is missing.`);
  process.exit(1);
}

if (dryRun) {
  console.log(`Dry run: would tag ${tag} and publish ${exePath} to GitHub releases.`);
  console.log(`\n${notes}`);
  process.exit(0);
}

const existingTags = spawnSync('git', ['tag', '--list', tag], { encoding: 'utf8' });
if (existingTags.stdout.trim() !== '') {
  console.error(`Tag ${tag} already exists. Wait a minute and rerun.`);
  process.exit(1);
}

run('git', ['tag', tag]);
run('git', ['push', 'origin', tag]);

const gh = findGh();
const repository = githubRepository();
run(gh, [
  'release',
  'create',
  tag,
  exePath,
  '--repo',
  repository,
  '--title',
  tag,
  '--notes',
  notes,
]);
