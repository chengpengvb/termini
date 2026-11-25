const { containsLineBreak, containsJsCommentOutsideQuotes, isBracketPreservingTransform } = require('./_textScan.cjs');

const JSX_OPENING_TAG_SINGLE_LINE_MAX_LENGTH = 250;

function trySelectOpeningTagNameText(src, node) {
  if (!node.name) {
    return null;
  }
  const nameText = src.getText(node.name);
  if (nameText == null) {
    return null;
  }
  const trimmed = nameText.trim();
  if (trimmed.length === 0) {
    return null;
  }
  return trimmed;
}

function trySelectSingleLineAttributesText(src, node) {
  const attrs = node.attributes;
  if (!Array.isArray(attrs)) {
    return null;
  }
  const parts = [];
  for (let i = 0; i < attrs.length; i += 1) {
    const a = attrs[i];
    const t = src.getText(a);
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
  return parts.join(' ');
}

function tryBuildSingleLineOpeningTag(src, node) {
  if (!node) {
    return null;
  }
  const nameText = trySelectOpeningTagNameText(src, node);
  if (nameText == null) {
    return null;
  }
  const attributesText = trySelectSingleLineAttributesText(src, node);
  if (attributesText == null) {
    return null;
  }
  let closerText = '>';
  if (node.selfClosing) {
    closerText = ' />';
  }
  if (attributesText.length === 0) {
    return '<' + nameText + closerText;
  }
  return '<' + nameText + ' ' + attributesText + closerText;
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
      JSXOpeningElement(node) {
        const raw = src.getText(node);
        if (!containsLineBreak(raw)) {
          return;
        }
        const formatted = tryBuildSingleLineOpeningTag(src, node);
        if (formatted == null) {
          return;
        }
        if (formatted.length > JSX_OPENING_TAG_SINGLE_LINE_MAX_LENGTH) {
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
          message: 'JSX opening tag should be in one line.',
          fix(fixer) {
            return fixer.replaceText(node, formatted);
          },
        });
      },
    };
  },
};

