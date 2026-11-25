function containsLineBreak(text) {
  if (text.indexOf('\n') >= 0) {
    return true;
  }
  if (text.indexOf('\r') >= 0) {
    return true;
  }
  return false;
}

function advanceInsideQuote(state, ch) {
  if (state.escaping) {
    state.escaping = false;
    return;
  }
  if (ch === '\\') {
    state.escaping = true;
    return;
  }
  if (state.mode === 'single' && ch === "'") {
    state.mode = 'code';
    return;
  }
  if (state.mode === 'double' && ch === '"') {
    state.mode = 'code';
    return;
  }
  if (state.mode === 'template' && ch === '`') {
    state.mode = 'code';
  }
}

function tryEnterQuote(state, ch) {
  if (ch === "'") {
    state.mode = 'single';
    state.escaping = false;
    return true;
  }
  if (ch === '"') {
    state.mode = 'double';
    state.escaping = false;
    return true;
  }
  if (ch === '`') {
    state.mode = 'template';
    state.escaping = false;
    return true;
  }
  return false;
}

function containsJsCommentOutsideQuotes(text) {
  const state = { mode: 'code', escaping: false };

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    let next = '';
    if (i + 1 < text.length) {
      next = text[i + 1];
    }

    if (state.mode !== 'code') {
      advanceInsideQuote(state, ch);
      continue;
    }

    if (ch === '/' && next === '/') {
      return true;
    }
    if (ch === '/' && next === '*') {
      return true;
    }
    tryEnterQuote(state, ch);
  }

  return false;
}

function countSyntaxBrackets(text) {
  const counts = { openCurly: 0, closeCurly: 0, openParen: 0, closeParen: 0, openSquare: 0, closeSquare: 0 };
  const state = { mode: 'code', escaping: false };

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (state.mode !== 'code') {
      advanceInsideQuote(state, ch);
      continue;
    }
    if (tryEnterQuote(state, ch)) {
      continue;
    }
    if (ch === '{') {
      counts.openCurly += 1;
    } else if (ch === '}') {
      counts.closeCurly += 1;
    } else if (ch === '(') {
      counts.openParen += 1;
    } else if (ch === ')') {
      counts.closeParen += 1;
    } else if (ch === '[') {
      counts.openSquare += 1;
    } else if (ch === ']') {
      counts.closeSquare += 1;
    }
  }

  return counts;
}

function bracketCountsEqual(a, b) {
  if (a.openCurly !== b.openCurly) {
    return false;
  }
  if (a.closeCurly !== b.closeCurly) {
    return false;
  }
  if (a.openParen !== b.openParen) {
    return false;
  }
  if (a.closeParen !== b.closeParen) {
    return false;
  }
  if (a.openSquare !== b.openSquare) {
    return false;
  }
  if (a.closeSquare !== b.closeSquare) {
    return false;
  }
  return true;
}

function isBracketPreservingTransform(original, transformed) {
  const a = countSyntaxBrackets(original);
  const b = countSyntaxBrackets(transformed);
  return bracketCountsEqual(a, b);
}

module.exports = {
  containsLineBreak,
  containsJsCommentOutsideQuotes,
  countSyntaxBrackets,
  bracketCountsEqual,
  isBracketPreservingTransform,
};

