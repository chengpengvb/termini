const fs = require('node:fs');
const path = require('node:path');
const childProcess = require('node:child_process');
const {
  formatRustUseImportsText,
  collapseRustTracingMacrosText,
} = require('../eslint/rules/rust-use-length-sort.cjs');

function isRustFile (filePath) {
  return filePath.endsWith('.rs');
}

function normalizeWindowsLongPathPrefix (inputPath) {
  if (process.platform !== 'win32') {
    return inputPath;
  }
  if (typeof inputPath !== 'string') {
    return inputPath;
  }
  if (inputPath.startsWith('\\\\?\\UNC\\')) {
    return '\\\\' + inputPath.slice('\\\\?\\UNC\\'.length);
  }
  if (inputPath.startsWith('\\\\?\\')) {
    return inputPath.slice('\\\\?\\'.length);
  }
  return inputPath;
}

function selectLineBreak (text) {
  if (text.indexOf('\r\n') >= 0) {
    return '\r\n';
  }
  return '\n';
}

function selectRustfmtFileHeaderPathOrNull (line) {
  if (typeof line !== 'string') {
    return null;
  }
  const matched = line.match(/^(.+\.rs):\s*$/);
  if (!matched) {
    return null;
  }
  return matched[1];
}

function buildRustfmtStdoutSections (stdout) {
  const lines = stdout.split(/\r\n|\n/);
  const headers = [];

  for (let i = 0; i < lines.length; i += 1) {
    const headerPath = selectRustfmtFileHeaderPathOrNull(lines[i]);
    if (!headerPath) {
      continue;
    }
    headers.push({ headerIndex: i, headerPath });
  }

  if (headers.length <= 0) {
    return { lines, sections: [] };
  }

  const sections = [];
  for (let i = 0; i < headers.length; i += 1) {
    const start = headers[i].headerIndex + 1;
    const end = i + 1 < headers.length ? headers[i + 1].headerIndex : lines.length;
    sections.push({
      headerPath: headers[i].headerPath,
      start,
      end,
    });
  }

  return { lines, sections };
}

function selectRustfmtFormattedTextForFileOrThrow (stdout, filePath) {
  const normalizedFilePath = path.resolve(normalizeWindowsLongPathPrefix(filePath));
  const result = buildRustfmtStdoutSections(stdout);

  if (result.sections.length <= 0) {
    return stdout;
  }

  for (let i = 0; i < result.sections.length; i += 1) {
    const section = result.sections[i];
    const normalizedHeaderPath = path.resolve(
      normalizeWindowsLongPathPrefix(section.headerPath),
    );
    if (normalizedHeaderPath !== normalizedFilePath) {
      continue;
    }

    let start = section.start;
    if (start < section.end && result.lines[start].trim().length <= 0) {
      start += 1;
    }
    return result.lines.slice(start, section.end).join('\n');
  }

  throw new Error(`rustfmt stdout missing target file: ${normalizedFilePath}`);
}

function selectParentDirOrNull (dirPath) {
  const parent = path.dirname(dirPath);
  if (parent === dirPath) {
    return null;
  }
  return parent;
}

function findRustemConfigOrNull (filePath) {
  let dirPath = path.dirname(path.resolve(filePath));
  while (dirPath) {
    const configPath = path.join(dirPath, 'rustfmt.toml');
    if (fs.existsSync(configPath)) {
      return configPath;
    }
    dirPath = selectParentDirOrNull(dirPath);
  }
  return null;
}

function findCargoTomlOrNull (filePath) {
  let dirPath = path.dirname(path.resolve(filePath));
  while (dirPath) {
    const cargoTomlPath = path.join(dirPath, 'Cargo.toml');
    if (fs.existsSync(cargoTomlPath)) {
      return cargoTomlPath;
    }
    dirPath = selectParentDirOrNull(dirPath);
  }
  return null;
}

function selectRustEditionOrNull (filePath) {
  const cargoTomlPath = findCargoTomlOrNull(filePath);
  if (!cargoTomlPath) {
    return null;
  }
  const cargoTomlText = fs.readFileSync(cargoTomlPath, 'utf8');
  const matched = cargoTomlText.match(/^\s*edition\s*=\s*"([^"]+)"/m);
  if (!matched) {
    return null;
  }
  return matched[1];
}

function runRustem (filePath) {
  const configPath = findRustemConfigOrNull(filePath);
  const edition = selectRustEditionOrNull(filePath);
  const args = ['--emit', 'stdout'];
  if (configPath) {
    args.push('--config-path', configPath);
  }
  if (edition) {
    args.push('--edition', edition);
  }
  args.push(filePath);

  const result = childProcess.spawnSync('rustfmt', args, { encoding: 'utf8' });
  if (result.status !== 0) {
    if (result.stdout) {
      process.stdout.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
    throw new Error(`rustfmt failed: ${filePath}`);
  }

  if (typeof result.stdout !== 'string') {
    throw new Error(`rustfmt stdout is not text: ${filePath}`);
  }

  const original = fs.readFileSync(filePath, 'utf8');
  const lineBreak = selectLineBreak(original);
  const selected = selectRustfmtFormattedTextForFileOrThrow(result.stdout, filePath);
  const formatted = selected.replace(/\r?\n/g, lineBreak);
  if (formatted !== original) {
    fs.writeFileSync(filePath, formatted, 'utf8');
    return true;
  }
  return false;
}

function formatUseImports (filePath) {
  const original = fs.readFileSync(filePath, 'utf8');
  const result = formatRustUseImportsText(original);
  if (!result.changed) {
    return false;
  }
  fs.writeFileSync(filePath, result.text, 'utf8');
  return true;
}

function collapseTracingMacros (filePath) {
  const original = fs.readFileSync(filePath, 'utf8');
  const result = collapseRustTracingMacrosText(original);
  if (!result.changed) {
    return false;
  }
  fs.writeFileSync(filePath, result.text, 'utf8');
  return true;
}

function isSkippableDirName (name) {
  const lowered = name.toLowerCase();
  return lowered === 'node_modules' || lowered === 'target' || lowered === '.git' || lowered === 'gen';
}

function collectRustFilesFromPath (inputPath, outFiles) {
  const stat = fs.statSync(inputPath);
  if (stat.isFile()) {
    if (isRustFile(inputPath)) {
      outFiles.push(inputPath);
    }
    return;
  }
  if (!stat.isDirectory()) {
    return;
  }
  const entries = fs.readdirSync(inputPath, { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = path.join(inputPath, entry.name);
    if (entry.isDirectory()) {
      if (isSkippableDirName(entry.name)) {
        continue;
      }
      collectRustFilesFromPath(entryPath, outFiles);
      continue;
    }
    if (entry.isFile() && isRustFile(entryPath)) {
      outFiles.push(entryPath);
    }
  }
}

function formatRustFile (filePath) {
  const normalizedPath = normalizeWindowsLongPathPrefix(filePath);
  const absolutePath = path.resolve(normalizedPath);
  if (!isRustFile(absolutePath)) {
    return false;
  }
  const rustfmtChanged = runRustem(absolutePath);
  const collapsed = collapseTracingMacros(absolutePath);
  const sorted = formatUseImports(absolutePath);
  return rustfmtChanged || collapsed || sorted;
}

function applyLengthSort (text) {
  const collapsed = collapseRustTracingMacrosText(text);
  const sorted = formatRustUseImportsText(collapsed.text);
  return sorted.text;
}

function selectStdinAnchorPath () {
  return path.join(__dirname, '..', 'src-tauri', 'dummy.rs');
}

function runRustemStdin (text) {
  const configPath = findRustemConfigOrNull(selectStdinAnchorPath());
  const edition = selectRustEditionOrNull(selectStdinAnchorPath());
  const args = [];
  if (configPath) {
    args.push('--config-path', configPath);
  }
  if (edition) {
    args.push('--edition', edition);
  }
  const result = childProcess.spawnSync('rustfmt', args, {
    encoding: 'utf8',
    input: text,
    maxBuffer: 32 * 1024 * 1024,
  });
  if (result.status !== 0) {
    if (result.stdout) {
      process.stderr.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
    throw new Error('rustfmt failed: stdin');
  }
  if (typeof result.stdout !== 'string') {
    throw new Error('rustfmt stdout is not text: stdin');
  }
  const lineBreak = selectLineBreak(text);
  return result.stdout.replace(/\r?\n/g, lineBreak);
}

function runStdin () {
  const chunks = [];
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    chunks.push(chunk);
  });
  process.stdin.on('end', () => {
    try {
      const original = chunks.join('');
      const rustfmtText = runRustemStdin(original);
      process.stdout.write(applyLengthSort(rustfmtText));
    } catch (error) {
      process.stderr.write(String(error) + '\n');
      process.exit(1);
    }
  });
}

function parseArgs (argv) {
  const rawArgs = argv.slice(2);
  const targets = [];
  let stdin = false;
  for (let i = 0; i < rawArgs.length; i += 1) {
    if (rawArgs[i] === '--stdin') {
      stdin = true;
      continue;
    }
    targets.push(normalizeWindowsLongPathPrefix(rawArgs[i]));
  }
  return { stdin, targets };
}

function run () {
  const parsed = parseArgs(process.argv);
  if (parsed.stdin) {
    runStdin();
    return;
  }
  const targets = parsed.targets;
  if (targets.length <= 0) {
    throw new Error('Missing Rust file or directory path.');
  }

  const files = [];
  for (let i = 0; i < targets.length; i += 1) {
    collectRustFilesFromPath(path.resolve(targets[i]), files);
  }
  files.sort((a, b) => a.localeCompare(b));
  if (files.length <= 0) {
    throw new Error('No .rs files found.');
  }

  let changedCount = 0;
  for (let i = 0; i < files.length; i += 1) {
    if (formatRustFile(files[i])) {
      changedCount += 1;
    }
  }
  console.log(`rust-format-file files=${files.length} changed=${changedCount}`);
}

try {
  run();
} catch (error) {
  console.error(String(error));
  process.exitCode = 1;
}
