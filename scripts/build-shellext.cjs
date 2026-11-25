// 编译 Windows Shell Extension，用于 App→OS 远程文件拖拽。
// 仅 Windows 平台需要；其他平台直接跳过。
// 流程：cargo clean 清理旧产物 -> cargo build --release -> 拷贝 DLL 到 src-tauri 的
// debug/release 输出目录（若已存在），确保下次启动主程序时能立即找到并自动注册。

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const DLL_NAME = 'shellext.dll';
const SHELLEXT_DIR = path.join(__dirname, '..', 'shellext');
const SRC_TAURI_TARGET = path.join(__dirname, '..', 'src-tauri', 'target');

function run(cmd, args, cwd) {
    console.log(`[build-shellext] $ ${cmd} ${args.join(' ')}`);
    const result = spawnSync(cmd, args, { cwd, stdio: 'inherit', shell: process.platform === 'win32' });
    if (result.status !== 0) {
        throw new Error(`[build-shellext] command failed: ${cmd} ${args.join(' ')}`);
    }
}

function copyIfTargetExists(srcFile, targetDir) {
    if (!fs.existsSync(targetDir)) {
        return;
    }
    const dest = path.join(targetDir, DLL_NAME);
    fs.copyFileSync(srcFile, dest);
    console.log(`[build-shellext] copied to ${dest}`);
}

function main() {
    if (process.platform !== 'win32') {
        console.log('[build-shellext] not Windows, skip.');
        return;
    }

    console.log('[build-shellext] cleaning old build artifacts...');
    run('cargo', ['clean'], SHELLEXT_DIR);

    console.log('[build-shellext] building release DLL...');
    run('cargo', ['build', '--release'], SHELLEXT_DIR);

    const builtDll = path.join(SHELLEXT_DIR, 'target', 'release', DLL_NAME);
    if (!fs.existsSync(builtDll)) {
        throw new Error(`[build-shellext] build succeeded but DLL not found at ${builtDll}`);
    }

    copyIfTargetExists(builtDll, path.join(SRC_TAURI_TARGET, 'debug'));
    copyIfTargetExists(builtDll, path.join(SRC_TAURI_TARGET, 'release'));

    console.log('[build-shellext] done.');
}

main();
