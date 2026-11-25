const { isBracketPreservingTransform } = require('./_textScan.cjs');

const RULE_MESSAGE = 'sx must be formatted to single line.';

function selectJsxAttributeName(node) {
  if (!node) {
    return null;
  }
  const nameNode = node.name;
  if (!nameNode) {
    return null;
  }
  if (typeof nameNode.name === 'string') {
    return nameNode.name;
  }
  return null;
}

function isWhitespaceChar(ch) {
  if (ch === ' ') {
    return true;
  }
  if (ch === '\t') {
    return true;
  }
  if (ch === '\n') {
    return true;
  }
  return ch === '\r';

}

function trimTrailingSpace(out) {
  let end = out.length;
  while (end > 0 && out[end - 1] === ' ') {
    end -= 1;
  }
  return out.slice(0, end);
}

function appendPendingSpaceIfNeeded(state, nextChar) {
  if (!state.pendingSpace) {
    return;
  }
  if (state.out.length === 0) {
    state.pendingSpace = false;
    return;
  }
  if (state.out.endsWith(' ')) {
    state.pendingSpace = false;
    return;
  }
  if (nextChar === '}' || nextChar === ',' || nextChar === ':' || nextChar === ')' || nextChar === ']') {
    state.pendingSpace = false;
    return;
  }
  if (nextChar === '{' && state.lastCodeChar === '{') {
    state.pendingSpace = false;
    return;
  }
  if (nextChar === '(' && state.lastCodeChar === '(') {
    state.pendingSpace = false;
    return;
  }
  state.out += ' ';
  state.pendingSpace = false;
}

function stripTrailingComma(state) {
  if (state.out.endsWith(',')) {
    state.out = state.out.slice(0, -1);
    if (state.out.length === 0) {
      state.lastCodeChar = '';
    } else {
      state.lastCodeChar = state.out[state.out.length - 1];
    }
  }
}

function appendRightBrace(state) {
  state.out = trimTrailingSpace(state.out);
  stripTrailingComma(state);
  if (state.lastCodeChar !== '{' && state.lastCodeChar !== '}') {
    if (state.out.length > 0 && !state.out.endsWith(' ')) {
      state.out += ' ';
    }
  }
  state.out += '}';
  state.lastCodeChar = '}';
  state.pendingSpace = false;
}

function appendRightBracket(state, closing) {
  state.out = trimTrailingSpace(state.out);
  stripTrailingComma(state);
  state.out += closing;
  state.lastCodeChar = closing;
  state.pendingSpace = false;
}

function buildFormatState() {
  return {
    out: '',
    pendingSpace: false,
    mode: 'code',
    escaping: false,
    lastCodeChar: '',
  };
}

function isCodeCommentStart(ch, next) {
  if (ch !== '/') {
    return false;
  }
  if (next === '/') {
    return true;
  }
  if (next === '*') {
    return true;
  }
  return false;
}

function tryConsumeWhitespace(state, ch) {
  if (!isWhitespaceChar(ch)) {
    return false;
  }
  state.pendingSpace = true;
  return true;
}

function tryConsumeQuoteStart(state, ch) {
  if (ch !== "'" && ch !== '"' && ch !== '`') {
    return false;
  }
  appendPendingSpaceIfNeeded(state, ch);
  state.out += ch;
  state.pendingSpace = false;
  state.lastCodeChar = ch;
  state.escaping = false;

  if (ch === "'") {
    state.mode = 'single';
    return true;
  }
  if (ch === '"') {
    state.mode = 'double';
    return true;
  }
  state.mode = 'template';
  return true;
}

function tryConsumePunctuation(state, ch) {
  if (ch === ',' || ch === ':') {
    state.out = trimTrailingSpace(state.out);
    state.out += ch;
    state.pendingSpace = true;
    state.lastCodeChar = ch;
    return true;
  }
  if (ch === '{') {
    appendPendingSpaceIfNeeded(state, ch);
    state.out += ch;
    state.pendingSpace = true;
    state.lastCodeChar = ch;
    return true;
  }
  if (ch === '}') {
    appendRightBrace(state);
    return true;
  }
  if (ch === ')' || ch === ']') {
    appendRightBracket(state, ch);
    return true;
  }
  return false;
}

function consumeCodeChar(state, ch, next) {
  if (isCodeCommentStart(ch, next)) {
    return null;
  }
  if (tryConsumeWhitespace(state, ch)) {
    return true;
  }
  if (tryConsumeQuoteStart(state, ch)) {
    return true;
  }
  if (tryConsumePunctuation(state, ch)) {
    return true;
  }

  appendPendingSpaceIfNeeded(state, ch);
  state.out += ch;
  state.pendingSpace = false;
  state.lastCodeChar = ch;
  return true;
}

function consumeSingleQuoteChar(state, ch) {
  if (ch === '\n' || ch === '\r') {
    return null;
  }
  state.out += ch;
  if (state.escaping) {
    state.escaping = false;
    return true;
  }
  if (ch === '\\') {
    state.escaping = true;
    return true;
  }
  if (ch === "'") {
    state.mode = 'code';
  }
  return true;
}

function consumeDoubleQuoteChar(state, ch) {
  if (ch === '\n' || ch === '\r') {
    return null;
  }
  state.out += ch;
  if (state.escaping) {
    state.escaping = false;
    return true;
  }
  if (ch === '\\') {
    state.escaping = true;
    return true;
  }
  if (ch === '"') {
    state.mode = 'code';
  }
  return true;
}

function consumeTemplateQuoteChar(state, ch) {
  if (ch === '\n' || ch === '\r') {
    return null;
  }
  state.out += ch;
  if (state.escaping) {
    state.escaping = false;
    return true;
  }
  if (ch === '\\') {
    state.escaping = true;
    return true;
  }
  if (ch === '`') {
    state.mode = 'code';
  }
  return true;
}

function consumeCharIntoState(state, ch, next) {
  if (state.mode === 'single') {
    return consumeSingleQuoteChar(state, ch);
  }
  if (state.mode === 'double') {
    return consumeDoubleQuoteChar(state, ch);
  }
  if (state.mode === 'template') {
    return consumeTemplateQuoteChar(state, ch);
  }
  return consumeCodeChar(state, ch, next);
}

function buildCodeCollapsedWhitespace(text) {
  const state = buildFormatState();
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    let next = '';
    if (i + 1 < text.length) {
      next = text[i + 1];
    }
    const ok = consumeCharIntoState(state, ch, next);
    if (ok == null) {
      return null;
    }
  }
  if (state.mode !== 'code') {
    return null;
  }
  return state.out;
}

function tryFormatSxAttributeToSingleLine(originalSx) {
  const result = buildCodeCollapsedWhitespace(originalSx);
  if (result == null) {
    return null;
  }
  return result.trim();
}

function isSxVariableName(name) {
  if (!name) {
    return false;
  }
  if (name.length < 3) {
    return false;
  }
  if (!name.endsWith('Sx')) {
    return false;
  }
  return true;
}

function reportSxVariableDeclarator(context, src, node) {
  if (!node.id) {
    return;
  }
  if (node.id.type !== 'Identifier') {
    return;
  }
  if (!isSxVariableName(node.id.name)) {
    return;
  }
  if (!node.init) {
    return;
  }
  const raw = src.getText(node.init);
  if (raw.indexOf('\n') < 0) {
    return;
  }
  const formatted = tryFormatSxAttributeToSingleLine(raw);
  if (formatted == null) {
    return;
  }
  if (formatted === raw) {
    return;
  }
  if (!isBracketPreservingTransform(raw, formatted)) {
    return;
  }
  context.report({
    node: node.init,
    message: RULE_MESSAGE,
    fix(fixer) {
      return fixer.replaceText(node.init, formatted);
    },
  });
}

module.exports = {
  meta: {
    type: 'suggestion',
    fixable: 'code',
    schema: [],
  },
  create(context) {
    const src = context.sourceCode;
    return {
      JSXAttribute(node) {
        const name = selectJsxAttributeName(node);
        if (name !== 'sx') {
          return;
        }
        const raw = src.getText(node);
        if (raw.indexOf('\n') < 0) {
          return;
        }
        if (raw.indexOf('sx={{') < 0) {
          return;
        }
        const formatted = tryFormatSxAttributeToSingleLine(raw);
        if (formatted == null) {
          return;
        }
        if (formatted === raw) {
          return;
        }
        if (!isBracketPreservingTransform(raw, formatted)) {
          return;
        }
        context.report({
          node,
          message: RULE_MESSAGE,
          fix(fixer) {
            return fixer.replaceText(node, formatted);
          },
        });
      },
    };
  },
};

