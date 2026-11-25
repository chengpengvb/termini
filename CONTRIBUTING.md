# Contributing to Termini

English | [简体中文](#简体中文)

Thanks for helping out. Bug reports, fixes, translations and features are all welcome.

## Before you start

- **Bugs and feature ideas**: open an [issue](https://github.com/chengpengvb/termini/issues/new/choose) first, so we can agree on the approach before you spend time on code.
- **Security issues**: do not open a public issue. See [SECURITY.md](SECURITY.md).
- Small fixes (typos, obvious bugs) can go straight to a pull request.

## Development setup

Follow [Build from source](README.md#build-from-source) in the README, then:

```bash
yarn
yarn tauri:dev
```

## Code style

The rules are enforced by ESLint and a Rust formatter; CI runs the same checks.

- Frontend: `yarn lint` must pass. Fix with `yarn format:ts`, or a single file with `yarn eslint:fix-file <file>`.
- Rust: run `yarn format:rs` before committing.
- `if` / `else` / loop bodies always use braces, including one-line returns.
- No CJK string literals in frontend code — user-facing text goes through `src/common/i18n/catalog/*`. Add the key to every locale file.
- Read the surrounding code and follow its patterns; reuse existing services and components instead of adding parallel ones.
- Do not disable lint rules with `eslint-disable`.

More conventions are in [AGENTS.md](AGENTS.md).

## Pull requests

1. Fork the repository and branch off `main`.
2. Keep each PR focused on one change.
3. Describe what changed, why, and how you tested it (platforms, protocols, devices).
4. UI changes: include a screenshot.
5. CI must be green: frontend lint and the secret scan. Run `cargo check` in `src-tauri` yourself for Rust changes.

## Never commit secrets

Signing keys, store credentials, API keys, device passwords and captured customer data must never enter the repository — not even in tests. Use environment variables for test credentials (see `TERMINI_TEST_RDP_PASSWORD`).

Every pull request is scanned; PRs containing credentials or key files (`*.p8`, `*.jks`, `*.p12`, `*.pem`, `*.keystore`, `*.mobileprovision`, …) are rejected. Changes to `.gitignore` and `.github/` require maintainer review.

## License

By contributing you agree that your contributions are licensed under the [GPL-3.0](LICENSE).

---

## 简体中文

感谢参与。Bug 报告、修复、翻译和新功能都欢迎。

### 开始之前

- **Bug 和功能建议**：先[开 Issue](https://github.com/chengpengvb/termini/issues/new/choose)，对齐方案再写代码，避免白做。
- **安全漏洞**：不要公开提交，见 [SECURITY.md](SECURITY.md)。
- 小修复（错别字、明显 Bug）可以直接提 PR。

### 开发环境

按 README 的[从源码编译](README.zh-CN.md#从源码编译)配好环境，然后：

```bash
yarn
yarn tauri:dev
```

### 代码规范

规范由 ESLint 和 Rust 格式化脚本强制，CI 跑同样的检查。

- 前端：`yarn lint` 必须通过，可用 `yarn format:ts` 或 `yarn eslint:fix-file <文件>` 修复。
- Rust：提交前运行 `yarn format:rs`。
- `if` / `else` / 循环体一律写大括号，单行 return 也不例外。
- 前端代码禁止中文等 CJK 字面量，界面文案统一走 `src/common/i18n/catalog/*`，新增 key 要补齐所有语言。
- 先读周边代码并沿用现有写法，复用已有 service 和组件，不要另起一套。
- 不允许用 `eslint-disable` 绕过规则。

更多约定见 [AGENTS.md](AGENTS.md)。

### Pull Request

1. Fork 仓库，从 `main` 拉分支。
2. 一个 PR 只做一件事。
3. 说明改了什么、为什么改、怎么测的（平台、协议、设备）。
4. 界面改动附截图。
5. CI 必须全绿：前端 lint 与密钥扫描。改了 Rust 请自行在 `src-tauri` 下跑 `cargo check`。

### 禁止提交敏感信息

签名密钥、商店凭据、API Key、设备密码、抓取的客户数据一律不得进仓库，测试代码也不行。测试凭据用环境变量（参考 `TERMINI_TEST_RDP_PASSWORD`）。

每个 PR 都会被扫描，包含凭据或密钥文件（`*.p8`、`*.jks`、`*.p12`、`*.pem`、`*.keystore`、`*.mobileprovision` 等）的 PR 会被拒绝。修改 `.gitignore` 与 `.github/` 需要维护者审核。

### 许可证

提交贡献即表示你同意以 [GPL-3.0](LICENSE) 授权你的代码。
