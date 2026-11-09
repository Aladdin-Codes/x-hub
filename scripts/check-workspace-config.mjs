import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const readJson = (relativePath) =>
  JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8'));

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

const workspace = readJson('x-hub.code-workspace');
const settings = readJson('.vscode/settings.json');
const tasks = readJson('.vscode/tasks.json');
const launch = readJson('.vscode/launch.json');
const manifest = readJson('package.json');
const lock = readJson('package-lock.json');

assert(
  workspace.folders.some((folder) => path.resolve(root, folder.path) === root),
  'The workspace file must open the project root.',
);
assert(
  settings['terminal.integrated.cwd'] === '${workspaceFolder}',
  'The integrated terminal must start at the workspace root.',
);
assert(
  tasks.options?.cwd === '${workspaceFolder}',
  'VS Code tasks must run from the workspace root.',
);

const taskLabels = new Set(tasks.tasks.map((task) => task.label));
assert(taskLabels.size === tasks.tasks.length, 'Task labels must be unique.');

for (const configuration of launch.configurations) {
  assert(
    taskLabels.has(configuration.preLaunchTask),
    `Unknown preLaunchTask: ${configuration.preLaunchTask}`,
  );
  assert(
    configuration.program?.startsWith('${workspaceFolder}/'),
    `Debugger program must use a workspace-relative path: ${configuration.name}`,
  );
}

for (const relativePath of [
  'x-hub.code-workspace',
  '.vscode/settings.json',
  '.vscode/tasks.json',
  '.vscode/launch.json',
  '.vscode/extensions.json',
]) {
  const contents = fs.readFileSync(path.join(root, relativePath), 'utf8');
  assert(
    !/[A-Z]:[\\/](?:Users|home)[\\/]/i.test(contents),
    `Do not commit a user-specific absolute path in ${relativePath}.`,
  );
}

assert(
  manifest.version === lock.packages?.['']?.version,
  'package.json and the lockfile must use the same project version.',
);

for (const dependencyType of ['dependencies', 'devDependencies']) {
  const declared = manifest[dependencyType] ?? {};
  const locked = lock.packages?.['']?.[dependencyType] ?? {};
  const missing = Object.keys(locked).filter((name) => !(name in declared));
  const extra = Object.keys(declared).filter((name) => !(name in locked));
  assert(
    missing.length === 0 && extra.length === 0,
    `${dependencyType} differ from package-lock.json (missing: ${missing.join(', ') || 'none'}; extra: ${extra.join(', ') || 'none'}).`,
  );
}

console.log('Workspace configuration is valid.');
