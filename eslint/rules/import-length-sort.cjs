const RULE_MESSAGE = 'Imports must be sorted by line length (short to long).';

function isOnlyWhitespaceBetweenImports(src, imports) {
  const fullText = src.getText();
  for (let i = 0; i < imports.length - 1; i += 1) {
    const endOfPrev = imports[i].range[1];
    const startOfNext = imports[i + 1].range[0];
    if (startOfNext <= endOfPrev) {
      continue;
    }
    const between = fullText.slice(endOfPrev, startOfNext);
    if (between.trim().length > 0) {
      return false;
    }
  }
  return true;
}

function selectSourceCodeOrNull(context) {
  if (context == null) {
    return null;
  }
  if (context.sourceCode != null) {
    return context.sourceCode;
  }
  if (typeof context.getSourceCode === 'function') {
    return context.getSourceCode();
  }
  return null;
}

function buildSortedImportTexts(src, importNodes) {
  const sortedNodes = [...importNodes].sort((a, b) => {
    const aText = src.getText(a).trim();
    const bText = src.getText(b).trim();
    const diff = aText.length - bText.length;
    if (diff !== 0) {
      return diff;
    }
    return aText.localeCompare(bText);
  });
  return sortedNodes.map((n) => src.getText(n).trim());
}

function buildOriginalImportTexts(src, importNodes) {
  return importNodes.map((n) => src.getText(n).trim());
}

function isDifferentTextArray(a, b) {
  if (a.length !== b.length) {
    return true;
  }
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) {
      return true;
    }
  }
  return false;
}

function selectLineBreak(fullText) {
  if (fullText.indexOf('\r\n') >= 0) {
    return '\r\n';
  }
  return '\n';
}

function selectTrailingWhitespaceEnd(fullText, startIndex) {
  const offset = fullText.slice(startIndex).search(/[^\s]/);
  if (offset < 0) {
    return fullText.length;
  }
  return startIndex + offset;
}

function selectProgramBodyOrEmpty(node) {
  if (node && Array.isArray(node.body)) {
    return node.body;
  }
  return [];
}

function selectImportDeclarations(nodes) {
  return nodes.filter((n) => n && n.type === 'ImportDeclaration');
}

function buildFixTextOrNull(src, importNodes, fullText, startIndex, endIndex) {
  const lineBreak = selectLineBreak(fullText);
  const next = buildSortedImportTexts(src, importNodes);
  const original = buildOriginalImportTexts(src, importNodes);

  const suffix = endIndex < fullText.length ? lineBreak + lineBreak : lineBreak;
  const fixText = next.join(lineBreak) + suffix;
  const originalText = fullText.slice(startIndex, endIndex);

  if (isDifferentTextArray(original, next) !== true && originalText === fixText) {
    return null;
  }
  return fixText;
}

module.exports = {
  meta: {
    type: 'suggestion',
    fixable: 'code',
    schema: [],
  },
  create(context) {
    const src = selectSourceCodeOrNull(context);
    return {
      Program(node) {
        if (src == null) {
          return;
        }
        const body = selectProgramBodyOrEmpty(node);
        const imports = selectImportDeclarations(body);
        if (imports.length <= 1) {
          return;
        }
        if (!isOnlyWhitespaceBetweenImports(src, imports)) {
          return;
        }
        const first = imports[0];
        const last = imports[imports.length - 1];
        const fullText = src.getText();
        const end = selectTrailingWhitespaceEnd(fullText, last.range[1]);
        const fixText = buildFixTextOrNull(
          src,
          imports,
          fullText,
          first.range[0],
          end,
        );
        if (fixText == null) {
          return;
        }
        context.report({
          node: first,
          message: RULE_MESSAGE,
          fix(fixer) {
            return fixer.replaceTextRange([first.range[0], end], fixText);
          },
        });
      },
    };
  },
};

