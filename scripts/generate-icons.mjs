import {execSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'path';
import {fileURLToPath} from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, '..');
const iconSrc = path.join(projectRoot, 'src', 'assets', 'icon', 'logo.svg');
const opaqueSrc = path.join(projectRoot, 'src-tauri', 'icons', '.icon-source.svg');
const ICON_BG = '#23272e';

const androidBgCandidates = [
    path.join(projectRoot, 'src-tauri', 'icons', 'android', 'values', 'ic_launcher_background.xml'),
    path.join(projectRoot, 'src-tauri', 'gen', 'android', 'app', 'src', 'main', 'res', 'values', 'ic_launcher_background.xml'),
];

/** iOS AppIcon.appiconset sizes from Contents.json (px = point * scale). */
const iosIcons = [
    {name: 'AppIcon-20x20@1x.png', px: 20},
    {name: 'AppIcon-20x20@2x.png', px: 40},
    {name: 'AppIcon-20x20@2x-1.png', px: 40},
    {name: 'AppIcon-20x20@3x.png', px: 60},
    {name: 'AppIcon-29x29@1x.png', px: 29},
    {name: 'AppIcon-29x29@2x.png', px: 58},
    {name: 'AppIcon-29x29@2x-1.png', px: 58},
    {name: 'AppIcon-29x29@3x.png', px: 87},
    {name: 'AppIcon-40x40@1x.png', px: 40},
    {name: 'AppIcon-40x40@2x.png', px: 80},
    {name: 'AppIcon-40x40@2x-1.png', px: 80},
    {name: 'AppIcon-40x40@3x.png', px: 120},
    {name: 'AppIcon-60x60@2x.png', px: 120},
    {name: 'AppIcon-60x60@3x.png', px: 180},
    {name: 'AppIcon-76x76@1x.png', px: 76},
    {name: 'AppIcon-76x76@2x.png', px: 152},
    {name: 'AppIcon-83.5x83.5@2x.png', px: 167},
    {name: 'AppIcon-512@2x.png', px: 1024},
];

if (!fs.existsSync(iconSrc)) {
    console.error(`Icon source not found: ${iconSrc}`);
    process.exit(1);
}

const logo = fs.readFileSync(iconSrc, 'utf8');
const open = logo.indexOf('>');
const close = logo.lastIndexOf('</svg>');
if (open < 0 || close < 0) {
    console.error('Invalid logo.svg');
    process.exit(1);
}

// Evidence: logo.svg corners are transparent (a=0). iOS AppIcon treats that as white → white frame.
// Composite onto an opaque full-bleed plate before any rasterization.
const inner = logo.slice(open + 1, close).replace(/<!--[\s\S]*?-->/g, '').trim();
const opaqueSvg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 128 128" fill="none">\n` +
    `  <rect width="128" height="128" fill="${ICON_BG}"/>\n` +
    `${inner}\n` +
    `</svg>\n`;

fs.mkdirSync(path.dirname(opaqueSrc), {recursive: true});
fs.writeFileSync(opaqueSrc, opaqueSvg, 'utf8');

// Desktop + Android keep using tauri icon from the original logo (Android already OK).
console.log('Generating Tauri icons from logo.svg (Android/desktop unchanged path) ...');
execSync(`npx tauri icon "${iconSrc}"`, {cwd: projectRoot, stdio: 'inherit', shell: true});

const bgXml = `<resources>\n  <color name="ic_launcher_background">${ICON_BG}</color>\n</resources>\n`;
for (const androidBg of androidBgCandidates) {
    if (fs.existsSync(path.dirname(androidBg))) {
        fs.mkdirSync(path.dirname(androidBg), {recursive: true});
        fs.writeFileSync(androidBg, bgXml, 'utf8');
        console.log(`Android adaptive background -> ${ICON_BG} (${path.relative(projectRoot, androidBg)})`);
    }
}

// iOS only: rasterize the opaque composite (no transparent corners).
const iosIconSrc = path.join(projectRoot, 'src-tauri', 'icons', 'ios');
const iosIconDst = path.join(projectRoot, 'src-tauri', 'gen', 'apple', 'Assets.xcassets', 'AppIcon.appiconset');
fs.mkdirSync(iosIconSrc, {recursive: true});

let rsvg;
try {
    rsvg = execSync('command -v rsvg-convert', {encoding: 'utf8'}).trim();
} catch {
    console.error('rsvg-convert not found; cannot regenerate iOS AppIcon');
    process.exit(1);
}

for (const {name, px} of iosIcons) {
    const out = path.join(iosIconSrc, name);
    execSync(`"${rsvg}" -w ${px} -h ${px} "${opaqueSrc}" -o "${out}"`, {stdio: 'inherit'});
}
console.log(`iOS icons rendered from opaque logo composite -> ${path.relative(projectRoot, iosIconSrc)}`);

if (fs.existsSync(iosIconDst)) {
    for (const {name} of iosIcons) {
        fs.copyFileSync(path.join(iosIconSrc, name), path.join(iosIconDst, name));
    }
    console.log(`iOS AppIcon synced -> ${path.relative(projectRoot, iosIconDst)}`);
}

console.log('Icons generated.');
