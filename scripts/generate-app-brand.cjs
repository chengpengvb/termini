const fs = require('node:fs');
const path = require('node:path');
const {execSync} = require('node:child_process');

const ROOT = path.join(__dirname, '..');
const PRODUCT_NAME = 'Termini';
const TAURI_CONF_PATH = path.join(ROOT, 'src-tauri', 'tauri.conf.json');
const STRINGS_OVERRIDE = path.join(ROOT, 'src-tauri', 'android-overrides', 'app', 'src', 'main', 'res', 'values', 'strings.xml');

function writeIfChanged(filePath, content) {
    if (fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8') === content) {
        return false;
    }
    fs.mkdirSync(path.dirname(filePath), {recursive: true});
    fs.writeFileSync(filePath, content, 'utf8');
    return true;
}

function writeTauriConf() {
    const conf = JSON.parse(fs.readFileSync(TAURI_CONF_PATH, 'utf8'));
    conf.productName = PRODUCT_NAME;
    if (conf.app?.windows?.[0]) {
        conf.app.windows[0].title = PRODUCT_NAME;
    }
    const content = `${JSON.stringify(conf, null, 2)}\n`;
    if (writeIfChanged(TAURI_CONF_PATH, content)) {
        console.log(`[generate-app-brand] productName/title -> ${PRODUCT_NAME}`);
        return;
    }
    console.log(`[generate-app-brand] tauri.conf.json unchanged (${PRODUCT_NAME})`);
}

function writeAndroidStrings() {
    const content =
        `<resources>\n` +
        `    <string name="app_name">${PRODUCT_NAME}</string>\n` +
        `    <string name="main_activity_title">${PRODUCT_NAME}</string>\n` +
        `</resources>\n`;
    if (writeIfChanged(STRINGS_OVERRIDE, content)) {
        console.log('[generate-app-brand] wrote android-overrides strings.xml');
        return;
    }
    console.log('[generate-app-brand] android-overrides strings.xml unchanged');
}

function generateIcons() {
    execSync('node scripts/generate-icons.mjs', {cwd: ROOT, stdio: 'inherit'});
}

const skipIcons = process.argv.includes('--skip-icons');
writeTauriConf();
writeAndroidStrings();
if (!skipIcons) {
    generateIcons();
} else {
    console.log('[generate-app-brand] skip icons');
}
