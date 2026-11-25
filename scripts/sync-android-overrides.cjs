// 把 src-tauri/android-overrides/ 下的可追踪定制源文件镜像同步到 src-tauri/gen/android/。
// 由 package.json 的 build / dev 在执行前调用（也就是 Tauri 的 beforeBuildCommand），
// 保证 `tauri android init` 重新生成项目后，定制的 MainActivity.kt 等文件仍然生效。
// 如果 src-tauri/gen/android/ 不存在（桌面构建 / 全新 checkout），脚本会静默 no-op。

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.join(__dirname, '..');
const SRC_ROOT = path.join(ROOT, 'src-tauri', 'android-overrides');
const DST_ROOT = path.join(ROOT, 'src-tauri', 'gen', 'android');

function listFiles(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            files.push(...listFiles(full));
            continue;
        }
        if (entry.isFile()) {
            files.push(full);
        }
    }
    return files;
}

function shouldSkip(relativePath) {
    return relativePath === 'README.md';
}

function syncFile(srcFile, relativePath) {
    const dstFile = path.join(DST_ROOT, relativePath);
    const dstDir = path.dirname(dstFile);
    fs.mkdirSync(dstDir, { recursive: true });
    const srcContent = fs.readFileSync(srcFile);
    if (fs.existsSync(dstFile)) {
        const dstContent = fs.readFileSync(dstFile);
        if (dstContent.equals(srcContent)) {
            return false;
        }
    }
    fs.writeFileSync(dstFile, srcContent);
    return true;
}

function main() {
    if (!fs.existsSync(SRC_ROOT)) {
        console.log('[sync-android-overrides] src-tauri/android-overrides not found, skip');
        return;
    }
    if (!fs.existsSync(DST_ROOT)) {
        console.log('[sync-android-overrides] src-tauri/gen/android not found, skip (run `tauri android init` first to enable Android target)');
        return;
    }
    const files = listFiles(SRC_ROOT);
    let writeCount = 0;
    for (const srcFile of files) {
        const relativePath = path.relative(SRC_ROOT, srcFile).split(path.sep).join('/');
        if (shouldSkip(relativePath)) {
            continue;
        }
        if (syncFile(srcFile, relativePath)) {
            writeCount += 1;
            console.log(`[sync-android-overrides] wrote ${relativePath}`);
        }
    }
    console.log(`[sync-android-overrides] done, ${writeCount} file(s) updated, ${files.length} tracked`);
}

main();
