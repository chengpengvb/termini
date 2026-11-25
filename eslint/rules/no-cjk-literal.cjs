const CJK = /[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\u3000-\u303f\uff00-\uffef]/;

function isCatalogFile(filename) {
  const normalized = filename.replace(/\\/g, '/');
  return normalized.includes('/common/i18n/');
}

module.exports = {
  meta: {
    type: 'problem',
    docs: {
      description: 'CJK text must live in the i18n catalogs; UI strings use translate()/t(), developer-facing errors and logs use English.',
    },
    schema: [],
    messages: {
      cjk: 'Hard-coded CJK text. Move it to common/i18n and use t()/translate(), or write developer-facing messages in English.',
    },
  },
  create(context) {
    const filename = context.filename || context.getFilename();
    if (isCatalogFile(filename)) {
      return {};
    }
    function check(node, text) {
      if (typeof text === 'string' && CJK.test(text)) {
        context.report({ node, messageId: 'cjk' });
      }
    }
    return {
      Literal(node) {
        check(node, node.value);
      },
      TemplateElement(node) {
        check(node, node.value.cooked);
      },
      JSXText(node) {
        check(node, node.value);
      },
    };
  },
};
