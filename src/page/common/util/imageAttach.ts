import {generateId} from '@/common/util/IdUtil.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';

export interface AttachedImage {
    id: string;
    url: string;
    mediaType: string;
    data: string;
}

const MAX_SOURCE_BYTES = 25 * 1024 * 1024;
const KEEP_ORIGINAL_MAX_BYTES = 7.5 * 1024 * 1024;
const KEEP_ORIGINAL_TYPES = new Set(['image/png', 'image/jpeg', 'image/webp', 'image/gif']);
const MAX_EDGE = 1568;
const COMPRESS_QUALITY = 0.85;

function parseImageDataUrl(url: string): AttachedImage | null {
    const commaIndex = url.indexOf(',');
    if (commaIndex < 0) {
        return null;
    }
    const header = url.slice(0, commaIndex);
    const data = url.slice(commaIndex + 1);
    const match = header.match(/^data:([^;]+);base64$/);
    if (!match) {
        return null;
    }
    if (isEmpty(data)) {
        return null;
    }
    return {id: generateId(), url: url, mediaType: match[1], data: data};
}

function readSourceDataUrl(file: File): Promise<string | null> {
    return new Promise((resolve) => {
        const reader = new FileReader();
        reader.onload = () => {
            const result = reader.result as string | null;
            resolve(result);
        };
        reader.onerror = () => {
            resolve(null);
        };
        reader.readAsDataURL(file);
    });
}

function loadImage(url: string): Promise<HTMLImageElement | null> {
    return new Promise((resolve) => {
        const img = new Image();
        img.onload = () => {
            resolve(img);
        };
        img.onerror = () => {
            resolve(null);
        };
        img.src = url;
    });
}

function resolveTargetSize(width: number, height: number): {width: number; height: number} {
    const longEdge = Math.max(width, height);
    if (longEdge <= MAX_EDGE) {
        return {width: width, height: height};
    }
    const scale = MAX_EDGE / longEdge;
    return {width: Math.round(width * scale), height: Math.round(height * scale)};
}

function compressWithCanvas(img: HTMLImageElement): string | null {
    const size = resolveTargetSize(img.width, img.height);
    const canvas = document.createElement('canvas');
    canvas.width = size.width;
    canvas.height = size.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
        return null;
    }
    ctx.drawImage(img, 0, 0, size.width, size.height);
    return canvas.toDataURL('image/jpeg', COMPRESS_QUALITY);
}

export async function resolveAttachedImage(file: File): Promise<AttachedImage | null> {
    if (file.size > MAX_SOURCE_BYTES) {
        return null;
    }
    const sourceUrl = await readSourceDataUrl(file);
    if (isEmpty(sourceUrl)) {
        return null;
    }
    if (KEEP_ORIGINAL_TYPES.has(file.type) && file.size <= KEEP_ORIGINAL_MAX_BYTES) {
        return parseImageDataUrl(sourceUrl);
    }
    const img = await loadImage(sourceUrl);
    if (!img) {
        return null;
    }
    const compressed = compressWithCanvas(img);
    if (isEmpty(compressed)) {
        return null;
    }
    return parseImageDataUrl(compressed);
}
