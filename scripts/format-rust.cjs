/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');

const {
  collapseRustTracingMacrosText,
  formatRustUseImportsText,
} = require('../eslint/rules/rust-use-length-sort.cjs');

function isRsFile(filePath) {
  return filePath.toLowerCase().endsWith('.rs');
}

function isSkippableDirName(name) {
  const lowered = name.toLowerCase();
  if (lowered === 'node_modules') {
    return true;
  }
  if (lowered === 'target') {
    return true;
  }
  if (lowered === '.git') {
    return true;
  }
  return false;
}

function collectRustFilesFromPath(inputPath, outFiles) {
  const stat = fs.statSync(inputPath);
  if (stat.isFile()) {
    if (isRsFile(inputPath)) {
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
    if (entry.isFile()) {
      if (isRsFile(entryPath)) {
        outFiles.push(entryPath);
      }
    }
  }
}

function formatRustText(text) {
  const collapsed = collapseRustTracingMacrosText(text);
  const sorted = formatRustUseImportsText(collapsed.text);
  return {
    changed: Boolean(collapsed.changed || sorted.changed),
    text: sorted.text,
  };
}

function parseArgs(argv) {
  const rawArgs = argv.slice(2);
  const targets = [];
  let check = false;

  for (const arg of rawArgs) {
    if (arg === '--check') {
      check = true;
      continue;
    }
    targets.push(arg);
  }

  if (targets.length <= 0) {
    targets.push('src-tauri/src');
  }

  return { targets, check };
}

function main() {
  const { targets, check } = parseArgs(process.argv);
  const files = [];

  for (const target of targets) {
    const resolved = path.resolve(process.cwd(), target);
    collectRustFilesFromPath(resolved, files);
  }

  files.sort((a, b) => a.localeCompare(b));

  let changedCount = 0;
  for (const filePath of files) {
    const original = fs.readFileSync(filePath, 'utf8');
    const formatted = formatRustText(original);
    if (!formatted.changed) {
      continue;
    }
    changedCount += 1;
    if (check) {
      continue;
    }
    fs.writeFileSync(filePath, formatted.text, 'utf8');
  }

  if (check) {
    if (changedCount > 0) {
      console.error(`[format:rs] ${changedCount} file(s) need formatting.`);
      process.exitCode = 1;
      return;
    }
    console.log('[format:rs] OK');
    return;
  }

  console.log(`[format:rs] formatted ${changedCount} file(s).`);
}

main();

