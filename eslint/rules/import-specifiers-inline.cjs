const { containsLineBreak, containsJsCommentOutsideQuotes, isBracketPreservingTransform } = require('./_textScan.cjs');

const IMPORT_SPECIFIERS_SINGLE_LINE_MAX_LENGTH = 250;

function selectImportStartText(node) {
  if (!node) {
    return null;
  }
  if (node.importKind === 'type') {
    return 'import type';
  }
  return 'import';
}

function trySelectImportSourceText(src, node) {
  if (!node.source) {
    return null;
  }
  const t = src.getText(node.source);
  if (t == null) {
    return null;
  }
  const trimmed = t.trim();
  if (trimmed.length === 0) {
    return null;
  }
  return trimmed;
}

function selectImportSemicolonText(src, node) {
  const raw = src.getText(node).trim();
  if (raw.endsWith(';')) {
    return ';';
  }
  return '';
}

function selectDefaultImportSpecifier(specifiers) {
  for (let i = 0; i < specifiers.length; i += 1) {
    const s = specifiers[i];
    if (s && s.type === 'ImportDefaultSpecifier') {
      return s;
    }
  }
  return null;
}

function selectNamespaceImportSpecifier(specifiers) {
  for (let i = 0; i < specifiers.length; i += 1) {
    const s = specifiers[i];
    if (s && s.type === 'ImportNamespaceSpecifier') {
      return s;
    }
  }
  return null;
}

function selectNamedImportSpecifiers(specifiers) {
  const out = [];
  for (let i = 0; i < specifiers.length; i += 1) {
    const s = specifiers[i];
    if (s && s.type === 'ImportSpecifier') {
      out.push(s);
    }
  }
  return out;
}

function buildNamedImportText(src, namedSpecs) {
  if (!Array.isArray(namedSpecs)) {
    return null;
  }
  if (namedSpecs.length === 0) {
    return '';
  }
  const parts = [];
  for (let i = 0; i < namedSpecs.length; i += 1) {
    const s = namedSpecs[i];
    const t = src.getText(s);
    if (t == null) {
      return null;
    }
    const trimmed = t.trim();
    if (trimmed.length === 0) {
      continue;
    }
    if (containsLineBreak(trimmed)) {
      return null;
    }
    if (containsJsCommentOutsideQuotes(trimmed)) {
      return null;
    }
    parts.push(trimmed);
  }
  return '{ ' + parts.join(', ') + ' }';
}

function tryBuildImportClauseText(src, node) {
  const specifiers = node.specifiers;
  if (!Array.isArray(specifiers)) {
    return null;
  }
  const defaultSpec = selectDefaultImportSpecifier(specifiers);
  const namespaceSpec = selectNamespaceImportSpecifier(specifiers);
  const namedSpecs = selectNamedImportSpecifiers(specifiers);

  const namedText = buildNamedImportText(src, namedSpecs);
  if (namedText == null) {
    return null;
  }

  const leftParts = [];
  if (defaultSpec != null) {
    leftParts.push(src.getText(defaultSpec).trim());
  }
  if (namespaceSpec != null) {
    leftParts.push(src.getText(namespaceSpec).trim());
  }

  const left = leftParts.filter((x) => x.length > 0).join(', ');
  if (left.length === 0) {
    return namedText;
  }
  if (namedText.length === 0) {
    return left;
  }
  return left + ', ' + namedText;
}

function tryBuildSingleLineImport(src, node) {
  const startText = selectImportStartText(node);
  if (startText == null) {
    return null;
  }
  const clauseText = tryBuildImportClauseText(src, node);
  if (clauseText == null) {
    return null;
  }
  const sourceText = trySelectImportSourceText(src, node);
  if (sourceText == null) {
    return null;
  }
  const semiText = selectImportSemicolonText(src, node);
  if (clauseText.length === 0) {
    return startText + ' ' + sourceText + semiText;
  }
  return startText + ' ' + clauseText + ' from ' + sourceText + semiText;
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
      ImportDeclaration(node) {
        const raw = src.getText(node);
        if (!containsLineBreak(raw)) {
          return;
        }
        if (containsJsCommentOutsideQuotes(raw)) {
          return;
        }
        const fixed = tryBuildSingleLineImport(src, node);
        if (fixed == null) {
          return;
        }
        if (fixed.length > IMPORT_SPECIFIERS_SINGLE_LINE_MAX_LENGTH) {
          return;
        }
        if (fixed === raw) {
          return;
        }
        if (!isBracketPreservingTransform(raw, fixed)) {
          return;
        }
        context.report({
          node,
          message: 'Import specifiers should be formatted to single line.',
          fix(fixer) {
            return fixer.replaceText(node, fixed);
          },
        });
      },
    };
  },
};

