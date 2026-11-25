# 代码门禁

- 修改前先阅读目标文件及周边代码，沿用用户已经修正的写法；不得把用户明确纠正过的风格重新写回去。
- `if`、`else if`、`else` 的语句体必须使用 `{}`，单行语句、提前 `return`、`throw` 和 `continue` 也不得省略。
- 循环语句同样遵守 ESLint 的 `curly: ['error', 'all']` 规则。
- 修改前端代码后运行 `npm run eslint:check-file -- <修改的文件>`；全量门禁使用 `npm run lint`。不得关闭、降级或使用 `eslint-disable` 绕过此规则。
