const fs = require('fs');
const path = require('path');
const { formatRustUseImportsText, collapseRustTracingMacrosText } = require('./rules/rust-use-length-sort.cjs');

const SKIP_DIRS = new Set(['target', 'gen', 'node_modules', '.git']);

function collectRustFiles(root, out) {
  const entries = fs.readdirSync(root, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(root, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) {
        continue;
      }
      collectRustFiles(full, out);
      continue;
    }
    if (entry.isFile() && entry.name.endsWith('.rs')) {
      out.push(full);
    }
  }
}

function formatText(text) {
  const sorted = formatRustUseImportsText(text);
  const collapsed = collapseRustTracingMacrosText(sorted.text);
  return { changed: sorted.changed || collapsed.changed, text: collapsed.text };
}

function main() {
  const args = process.argv.slice(2);
  const write = args.includes('--write');
  const targets = args.filter((arg) => arg !== '--write');
  if (targets.length <= 0) {
    console.error('usage: node rust-format-runner.cjs [--write] <file-or-dir>...');
    process.exit(1);
  }

  const files = [];
  for (const target of targets) {
    const stat = fs.statSync(target);
    if (stat.isDirectory()) {
      collectRustFiles(target, files);
    } else {
      files.push(target);
    }
  }

  let changedCount = 0;
  for (const file of files) {
    const before = fs.readFileSync(file, 'utf8');
    const result = formatText(before);
    if (!result.changed || result.text === before) {
      continue;
    }
    changedCount += 1;
    if (write) {
      fs.writeFileSync(file, result.text, 'utf8');
      console.log(`fixed: ${file}`);
    } else {
      console.log(`would fix: ${file}`);
    }
  }
  console.log(`scanned=${files.length} changed=${changedCount} mode=${write ? 'write' : 'dry-run'}`);
}

main();
