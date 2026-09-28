import { afterEach, expect, test } from 'bun:test';
import { YAML } from 'bun';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const project = fileURLToPath(new URL('..', import.meta.url));
const temporaryRoot = join(project, 'src-tauri/target');
const workspaces = [];
const workflow = YAML.parse(
  readFileSync(join(project, '.github/workflows/prepare-release.yml'), 'utf8')
);
const commitStep = workflow.jobs.prepare.steps.find(
  (step) => step.name === 'commit, tag & push'
).run;

function command(cwd, args, version = '1.2.4') {
  return spawnSync(args[0], args.slice(1), {
    cwd,
    encoding: 'utf8',
    env: {
      ...process.env,
      TMPDIR: cwd,
      GIT_CONFIG_NOSYSTEM: '1',
      GIT_CONFIG_GLOBAL: '/dev/null',
      GIT_TERMINAL_PROMPT: '0',
      VERSION: version,
    },
  });
}

function succeeds(cwd, args, version) {
  const result = command(cwd, args, version);
  expect(result.status, result.stderr || result.stdout).toBe(0);
  return result.stdout.trim();
}

function fixture(version = '1.2.3') {
  mkdirSync(temporaryRoot, { recursive: true });
  const root = mkdtempSync(join(temporaryRoot, 'release-prepare-'));
  workspaces.push(root);
  const repository = join(root, 'repository');
  mkdirSync(join(repository, 'scripts'), { recursive: true });
  mkdirSync(join(repository, 'src-tauri'));
  for (const script of ['release.sh', 'check-versions.sh', 'version-utils.sh']) {
    cpSync(join(project, 'scripts', script), join(repository, 'scripts', script));
  }
  for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
    writeFileSync(
      join(repository, file),
      JSON.stringify({ name: 'clipman', version }, null, 2) + '\n'
    );
  }
  writeFileSync(
    join(repository, 'src-tauri/Cargo.toml'),
    `[package]\nname = "clipman"\nversion = "${version}"\n`
  );
  writeFileSync(
    join(repository, 'src-tauri/Cargo.lock'),
    `[[package]]\nname = "clipman"\nversion = "${version}"\n`
  );
  for (const file of ['README.md', 'README_EN.md']) {
    writeFileSync(join(repository, file), `ClipMan_${version}_aarch64.dmg\n`);
  }
  writeFileSync(join(repository, 'release_notes_1.2.4.md'), '## 修复\n\n- 保存富文本。\n');
  succeeds(repository, ['git', 'init', '-b', 'main']);
  succeeds(repository, ['git', 'config', 'user.name', 'Release Test']);
  succeeds(repository, ['git', 'config', 'user.email', 'release-test@example.invalid']);
  succeeds(repository, ['git', 'add', '.']);
  succeeds(repository, ['git', 'commit', '-m', 'initial']);
  return { root, repository };
}

function remoteFor(root, repository) {
  const remote = join(root, 'remote.git');
  succeeds(root, ['git', 'init', '--bare', remote]);
  succeeds(repository, ['git', 'remote', 'add', 'origin', remote]);
  succeeds(repository, ['git', 'push', 'origin', 'main']);
  return remote;
}

afterEach(() => {
  for (const workspace of workspaces.splice(0)) rmSync(workspace, { recursive: true, force: true });
});

test('invalid, older and existing-tag versions leave release files unchanged', () => {
  const { repository } = fixture();
  succeeds(repository, ['git', 'tag', 'v1.2.4']);
  for (const version of [
    '01.2.4',
    '1.02.4',
    '1.2.04',
    'v1.2.4',
    '1.2',
    '1.2.4\n1.2.5',
    '1.2.2',
    '1.1.9',
    '0.99.99',
    '1.2.4',
  ]) {
    const result = command(repository, ['bash', 'scripts/release.sh', version]);
    expect(result.status).not.toBe(0);
    expect(succeeds(repository, ['git', 'status', '--porcelain'])).toBe('');
  }
  expect(command(repository, ['bash', 'scripts/release.sh', '1.2.4']).stderr).toContain(
    'Release 工作流'
  );
});

test('valid same-version reruns preserve notes and manifest consistency', () => {
  const { repository } = fixture();
  succeeds(repository, ['bash', 'scripts/release.sh', '1.2.4']);
  const firstDiff = succeeds(repository, ['git', 'diff']);
  succeeds(repository, ['bash', 'scripts/release.sh', '1.2.4']);
  expect(succeeds(repository, ['git', 'diff'])).toBe(firstDiff);
  expect(readFileSync(join(repository, 'release_notes_1.2.4.md'), 'utf8')).toBe(
    '## 修复\n\n- 保存富文本。\n'
  );
  succeeds(repository, ['bash', 'scripts/check-versions.sh', 'v1.2.4']);
  succeeds(repository, ['bash', 'scripts/release.sh', '1.10.0']);
  succeeds(repository, ['bash', 'scripts/check-versions.sh', '1.10.0']);
});

test('equal malformed manifest versions still fail validation', () => {
  const { repository } = fixture('01.2.3');
  expect(command(repository, ['bash', 'scripts/check-versions.sh']).status).not.toBe(0);
});

test('prepare pushes one release commit and tag without staging unrelated files', () => {
  const { root, repository } = fixture();
  const remote = remoteFor(root, repository);
  succeeds(repository, ['bash', 'scripts/release.sh', '1.2.4']);
  writeFileSync(join(repository, 'unrelated.txt'), '保留本地文件\n');
  succeeds(repository, ['bash', '-euo', 'pipefail', '-c', commitStep]);
  const head = succeeds(repository, ['git', 'rev-parse', 'HEAD']);
  expect(succeeds(remote, ['git', 'rev-parse', 'refs/heads/main'])).toBe(head);
  expect(succeeds(remote, ['git', 'rev-parse', 'refs/tags/v1.2.4'])).toBe(head);
  expect(succeeds(repository, ['git', 'status', '--porcelain'])).toBe('?? unrelated.txt');
  expect(command(repository, ['bash', 'scripts/release.sh', '1.2.4']).status).not.toBe(0);
  expect(workflow.concurrency).toEqual({ group: 'prepare-release', 'cancel-in-progress': false });
});

test('a competing remote tag rejects the entire atomic push', () => {
  const { root, repository } = fixture();
  const remote = remoteFor(root, repository);
  const original = succeeds(remote, ['git', 'rev-parse', 'refs/heads/main']);
  succeeds(repository, ['bash', 'scripts/release.sh', '1.2.4']);
  // 标签在预检查后出现，模拟其他发布过程抢先推送。
  succeeds(remote, ['git', 'tag', 'v1.2.4', original]);
  expect(command(repository, ['bash', '-euo', 'pipefail', '-c', commitStep]).status).not.toBe(0);
  expect(succeeds(remote, ['git', 'rev-parse', 'refs/heads/main'])).toBe(original);
  expect(succeeds(remote, ['git', 'rev-parse', 'refs/tags/v1.2.4'])).toBe(original);
});
