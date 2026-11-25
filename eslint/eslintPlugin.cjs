const importLengthSortPlugin = require('./rules/import-length-sort.cjs');
const importSpecifiersInlineRule = require('./rules/import-specifiers-inline.cjs');
const jsxOpeningTagInlineRule = require('./rules/jsx-opening-tag-inline.cjs');
const sxInlineFormatRule = require('./rules/sx-inline-format.cjs');
const noCjkLiteralRule = require('./rules/no-cjk-literal.cjs');

function selectRuleOrThrow(input, ruleName) {
  if (!input) {
    throw new Error(`Rule not found: ${ruleName}`);
  }
  if (typeof input.create !== 'function') {
    throw new Error(`Invalid rule: ${ruleName}`);
  }
  return input;
}

module.exports = {
  rules: {
    'import-length-sort': selectRuleOrThrow(
      importLengthSortPlugin,
      'import-length-sort',
    ),
    'import-specifiers-inline': selectRuleOrThrow(
      importSpecifiersInlineRule,
      'import-specifiers-inline',
    ),
    'jsx-opening-tag-inline': selectRuleOrThrow(
      jsxOpeningTagInlineRule,
      'jsx-opening-tag-inline',
    ),
    'sx-inline-format': selectRuleOrThrow(
      sxInlineFormatRule,
      'sx-inline-format',
    ),
    'no-cjk-literal': selectRuleOrThrow(
      noCjkLiteralRule,
      'no-cjk-literal',
    ),
  },
};

