export function resolveContrastForeground(background: string): string {
    if (!/^#[\da-f]{6}$/i.test(background)) {
        throw new Error(`contrast background must be a six-digit hex color: ${background}`);
    }
    const channels = [1, 3, 5].map(index => Number.parseInt(background.slice(index, index + 2), 16) / 255);
    const linear = channels.map(channel => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
    const luminance = linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
    const blackContrast = (luminance + 0.05) / 0.05;
    const whiteContrast = 1.05 / (luminance + 0.05);
    return blackContrast >= whiteContrast ? '#000000' : '#ffffff';
}
