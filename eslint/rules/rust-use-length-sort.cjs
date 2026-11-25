const RULE_MESSAGE = 'Rust use/mod declarations must be sorted by line length (short to long).';
const TRACING_COLLAPSE_MESSAGE = 'Rust tracing!/log!-style macro calls must be written on a single line.';
const DECL_LINE_PATTERN = /^(\s*)(?:pub(?:\s*\([^)]*\))?\s+)?(use|mod)\s+.+;\s*$/;
const TRACING_MACRO_NAMES = ['trace', 'debug', 'info', 'warn', 'error'];
const IDENT_CHAR_PATTERN = /[A-Za-z0-9_]/;
const WHITESPACE_PATTERN = /\s/;

function selectLineBreak(text) {
  if (text.indexOf('\r\n') >= 0) {
    return '\r\n';
  }
  return '\n';
}

function isBlankLine(line) {
  return line.trim().length <= 0;
}

function classifyDeclLine(line) {
  if (line.indexOf('//') >= 0) {
    return null;
  }
  if (line.indexOf('/*') >= 0) {
    return null;
  }
  const matched = line.match(DECL_LINE_PATTERN);
  if (!matched) {
    return null;
  }
  return { indent: matched[1], kind: matched[2] };
}

function compareUseTextByLength(a, b) {
  const lengthDiff = a.length - b.length;
  if (lengthDiff !== 0) {
    return lengthDiff;
  }
  return a.localeCompare(b);
}

function leadingIndent(line) {
  return line.slice(0, line.length - line.trimStart().length);
}

// 单行外部属性(如 #[cfg(...)])：必须整行在同一行闭合，才安全地与下方声明绑定
function classifyAttrLine(line) {
  if (line.indexOf('//') >= 0 || line.indexOf('/*') >= 0) {
    return null;
  }
  const trimmed = line.trim();
  if (trimmed.startsWith('#[') && trimmed.endsWith(']')) {
    return { indent: leadingIndent(line) };
  }
  return null;
}

// 从 startIndex 解析一个「条目」：零或多行紧邻的属性 + 一行 use/mod 声明。
// 属性与声明之间不允许空行，保证属性始终粘在它标注的声明上。
function parseEntryAt(lines, startIndex) {
  let index = startIndex;
  const attrs = [];
  while (index < lines.length && classifyAttrLine(lines[index])) {
    attrs.push(lines[index]);
    index += 1;
  }
  if (index >= lines.length) {
    return null;
  }
  const declInfo = classifyDeclLine(lines[index]);
  if (!declInfo) {
    return null;
  }
  return { attrs, declLine: lines[index], declInfo, endIndex: index + 1 };
}

// 收集同种类(use/mod)的连续条目区域，允许条目间空行分隔(与旧行为一致，排序时空行会被合并)
function findEntryRegion(lines, startIndex, kind) {
  const entries = [];
  let index = startIndex;
  let endIndex = startIndex;

  while (index < lines.length) {
    const entry = parseEntryAt(lines, index);
    if (entry && entry.declInfo.kind === kind) {
      entries.push(entry);
      endIndex = entry.endIndex;
      index = entry.endIndex;
      continue;
    }

    if (!isBlankLine(lines[index])) {
      break;
    }

    let nextIndex = index + 1;
    while (nextIndex < lines.length && isBlankLine(lines[nextIndex])) {
      nextIndex += 1;
    }
    if (nextIndex >= lines.length) {
      break;
    }
    const nextEntry = parseEntryAt(lines, nextIndex);
    if (!nextEntry || nextEntry.declInfo.kind !== kind) {
      break;
    }
    index = nextIndex;
  }

  return { entries, endIndex };
}

// 按声明行长度排序条目；属性行随声明整体移动。缩进不一致则放弃排序(返回 null)
function buildSortedEntryLinesOrNull(entries) {
  if (entries.length <= 0) {
    return null;
  }
  let indent = null;
  for (let i = 0; i < entries.length; i += 1) {
    if (indent === null) {
      indent = entries[i].declInfo.indent;
    } else if (entries[i].declInfo.indent !== indent) {
      return null;
    }
  }

  const sortedEntries = [...entries].sort((a, b) => compareUseTextByLength(a.declLine.trim(), b.declLine.trim()));
  const outputLines = [];
  for (let i = 0; i < sortedEntries.length; i += 1) {
    const entry = sortedEntries[i];
    for (let j = 0; j < entry.attrs.length; j += 1) {
      outputLines.push(entry.attrs[j]);
    }
    outputLines.push(indent + entry.declLine.trim());
  }
  return outputLines;
}

function isSameLines(a, b) {
  if (a.length !== b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) {
      return false;
    }
  }
  return true;
}

function formatRustUseImportsText(text) {
  const lineBreak = selectLineBreak(text);
  const lines = text.split(/\r\n|\n/);
  const outputLines = [];
  let index = 0;
  let changed = false;

  while (index < lines.length) {
    const entry = parseEntryAt(lines, index);
    if (!entry) {
      outputLines.push(lines[index]);
      index += 1;
      continue;
    }

    const region = findEntryRegion(lines, index, entry.declInfo.kind);
    const sortedLines = buildSortedEntryLinesOrNull(region.entries);
    if (region.entries.length <= 1 || !sortedLines) {
      for (let i = index; i < region.endIndex; i += 1) {
        outputLines.push(lines[i]);
      }
      index = region.endIndex;
      continue;
    }

    const originalLines = [];
    for (let i = 0; i < region.entries.length; i += 1) {
      const current = region.entries[i];
      for (let j = 0; j < current.attrs.length; j += 1) {
        originalLines.push(current.attrs[j]);
      }
      originalLines.push(current.declLine);
    }
    if (isSameLines(originalLines, sortedLines)) {
      for (let i = index; i < region.endIndex; i += 1) {
        outputLines.push(lines[i]);
      }
      index = region.endIndex;
      continue;
    }

    changed = true;
    for (let i = 0; i < sortedLines.length; i += 1) {
      outputLines.push(sortedLines[i]);
    }
    index = region.endIndex;
  }

  return { changed, text: outputLines.join(lineBreak) };
}

// ── Rust 词法跳过：字符串 / 字符 / 裸字符串 / 注释 ──────────────────

function selectRegularStringEnd(text, startIndex, quote) {
  let i = startIndex + 1;
  while (i < text.length) {
    if (text[i] === '\\') {
      i += 2;
      continue;
    }
    if (text[i] === quote) {
      return i + 1;
    }
    i += 1;
  }
  return text.length;
}

function selectRawStringEndOrNull(text, startIndex) {
  let prefixLen = 0;
  if (text[startIndex] === 'r') {
    prefixLen = 1;
  } else if (text[startIndex] === 'b' && text[startIndex + 1] === 'r') {
    prefixLen = 2;
  } else {
    return null;
  }
  let cursor = startIndex + prefixLen;
  let hashes = 0;
  while (text[cursor] === '#') {
    hashes += 1;
    cursor += 1;
  }
  if (text[cursor] !== '"') {
    return null;
  }
  cursor += 1;
  const closeMark = '"' + '#'.repeat(hashes);
  while (cursor < text.length) {
    if (text.slice(cursor, cursor + closeMark.length) === closeMark) {
      return cursor + closeMark.length;
    }
    cursor += 1;
  }
  return text.length;
}

function selectCharEndOrNull(text, startIndex) {
  if (text[startIndex] !== "'") {
    return null;
  }
  let cursor = startIndex + 1;
  if (text[cursor] === '\\') {
    cursor += 1;
    if (text[cursor] === 'u' && text[cursor + 1] === '{') {
      cursor += 2;
      while (cursor < text.length && text[cursor] !== '}') {
        cursor += 1;
      }
      cursor += 1;
    } else {
      cursor += 1;
    }
  } else {
    cursor += 1;
  }
  if (text[cursor] === "'") {
    return cursor + 1;
  }
  return null;
}

function selectLifetimeEnd(text, startIndex) {
  let cursor = startIndex + 1;
  while (cursor < text.length && IDENT_CHAR_PATTERN.test(text[cursor])) {
    cursor += 1;
  }
  return cursor;
}

function selectLineCommentEnd(text, startIndex) {
  let cursor = startIndex + 2;
  while (cursor < text.length && text[cursor] !== '\n') {
    cursor += 1;
  }
  return cursor;
}

function selectBlockCommentEnd(text, startIndex) {
  let cursor = startIndex + 2;
  let depth = 1;
  while (cursor < text.length && depth > 0) {
    if (text[cursor] === '/' && text[cursor + 1] === '*') {
      depth += 1;
      cursor += 2;
      continue;
    }
    if (text[cursor] === '*' && text[cursor + 1] === '/') {
      depth -= 1;
      cursor += 2;
      continue;
    }
    cursor += 1;
  }
  return cursor;
}

function selectSkippableSpanEnd(text, startIndex) {
  const ch = text[startIndex];
  if (ch === '/' && text[startIndex + 1] === '/') {
    return { kind: 'line-comment', end: selectLineCommentEnd(text, startIndex) };
  }
  if (ch === '/' && text[startIndex + 1] === '*') {
    return { kind: 'block-comment', end: selectBlockCommentEnd(text, startIndex) };
  }
  const rawEnd = selectRawStringEndOrNull(text, startIndex);
  if (rawEnd !== null) {
    return { kind: 'raw-string', end: rawEnd };
  }
  if (ch === 'b' && text[startIndex + 1] === '"') {
    return { kind: 'byte-string', end: selectRegularStringEnd(text, startIndex + 1, '"') };
  }
  if (ch === '"') {
    return { kind: 'string', end: selectRegularStringEnd(text, startIndex, '"') };
  }
  if (ch === "'") {
    const charEnd = selectCharEndOrNull(text, startIndex);
    if (charEnd !== null) {
      return { kind: 'char', end: charEnd };
    }
    return { kind: 'lifetime', end: selectLifetimeEnd(text, startIndex) };
  }
  return null;
}

// ── tracing!/log! 宏调用合并 ──────────────────────────────────────

function isWordBoundaryBefore(text, index) {
  if (index <= 0) {
    return true;
  }
  return !IDENT_CHAR_PATTERN.test(text[index - 1]);
}

function matchTracingMacroAt(text, index) {
  if (!isWordBoundaryBefore(text, index)) {
    return null;
  }
  if (text.slice(index, index + 9) !== 'tracing::') {
    return null;
  }
  let cursor = index + 9;
  let matchedName = null;
  for (let i = 0; i < TRACING_MACRO_NAMES.length; i += 1) {
    const name = TRACING_MACRO_NAMES[i];
    if (text.slice(cursor, cursor + name.length) !== name) {
      continue;
    }
    if (text[cursor + name.length] !== '!') {
      continue;
    }
    matchedName = name;
    cursor += name.length + 1;
    break;
  }
  if (matchedName === null) {
    return null;
  }
  while (cursor < text.length && WHITESPACE_PATTERN.test(text[cursor])) {
    cursor += 1;
  }
  if (text[cursor] !== '(') {
    return null;
  }
  return { bodyStart: cursor + 1 };
}

function findBalancedParenClose(text, bodyStart) {
  let cursor = bodyStart;
  let depth = 1;
  while (cursor < text.length) {
    const skip = selectSkippableSpanEnd(text, cursor);
    if (skip !== null) {
      cursor = skip.end;
      continue;
    }
    if (text[cursor] === '(') {
      depth += 1;
      cursor += 1;
      continue;
    }
    if (text[cursor] === ')') {
      depth -= 1;
      if (depth === 0) {
        return cursor;
      }
      cursor += 1;
      continue;
    }
    cursor += 1;
  }
  return null;
}

function bodyContainsLineComment(body) {
  let cursor = 0;
  while (cursor < body.length) {
    const skip = selectSkippableSpanEnd(body, cursor);
    if (skip !== null) {
      if (skip.kind === 'line-comment') {
        return true;
      }
      cursor = skip.end;
      continue;
    }
    cursor += 1;
  }
  return false;
}

function collapseBodyWhitespace(body) {
  let cursor = 0;
  let out = '';
  while (cursor < body.length) {
    const skip = selectSkippableSpanEnd(body, cursor);
    if (skip !== null) {
      out += body.slice(cursor, skip.end);
      cursor = skip.end;
      continue;
    }
    const ch = body[cursor];
    if (WHITESPACE_PATTERN.test(ch)) {
      let next = cursor + 1;
      while (next < body.length && WHITESPACE_PATTERN.test(body[next])) {
        const nextSkip = selectSkippableSpanEnd(body, next);
        if (nextSkip !== null) {
          break;
        }
        next += 1;
      }
      out += ' ';
      cursor = next;
      continue;
    }
    out += ch;
    cursor += 1;
  }
  return out.trim();
}

function collapseRustTracingMacrosText(text) {
  let cursor = 0;
  let out = '';
  let changed = false;

  while (cursor < text.length) {
    const skip = selectSkippableSpanEnd(text, cursor);
    if (skip !== null) {
      out += text.slice(cursor, skip.end);
      cursor = skip.end;
      continue;
    }

    const matched = matchTracingMacroAt(text, cursor);
    if (matched === null) {
      out += text[cursor];
      cursor += 1;
      continue;
    }

    const bodyStart = matched.bodyStart;
    const bodyEnd = findBalancedParenClose(text, bodyStart);
    if (bodyEnd === null) {
      out += text.slice(cursor, bodyStart);
      cursor = bodyStart;
      continue;
    }

    const body = text.slice(bodyStart, bodyEnd);
    if (body.indexOf('\n') < 0) {
      out += text.slice(cursor, bodyEnd + 1);
      cursor = bodyEnd + 1;
      continue;
    }
    if (bodyContainsLineComment(body)) {
      out += text.slice(cursor, bodyEnd + 1);
      cursor = bodyEnd + 1;
      continue;
    }

    out += text.slice(cursor, bodyStart);
    out += collapseBodyWhitespace(body);
    out += ')';
    cursor = bodyEnd + 1;
    changed = true;
  }

  return { changed, text: out };
}

module.exports = {
  RULE_MESSAGE,
  TRACING_COLLAPSE_MESSAGE,
  formatRustUseImportsText,
  collapseRustTracingMacrosText,
};
