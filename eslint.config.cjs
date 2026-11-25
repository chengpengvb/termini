const tsParser = require('@typescript-eslint/parser');
const simpleImportSort = require('eslint-plugin-simple-import-sort');

const terminiPlugin = require('./eslint/eslintPlugin.cjs');

module.exports = [
    {
        files: ['src/**/*.{ts,tsx,js,jsx}'],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                ecmaVersion: 'latest',
                sourceType: 'module',
                ecmaFeatures: {jsx: true},
            },
        },
        plugins: {
            'simple-import-sort': simpleImportSort,
            termini: terminiPlugin,
        },
        rules: {
            curly: ['error', 'all'],
            'simple-import-sort/imports': 'off',
            'simple-import-sort/exports': 'error',
            'termini/import-length-sort': 'error',
            'termini/import-specifiers-inline': 'error',
            'termini/jsx-opening-tag-inline': 'error',
            'termini/sx-inline-format': 'error',
            'termini/no-cjk-literal': 'error',
        },
    },
];

