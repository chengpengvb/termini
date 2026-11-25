import {type RefObject, useRef, useState} from 'react';
import {requireRdpCanvas} from '@/page/common/util/rdpUtil.ts';
import {RDP_CURSOR_KIND} from '@/common/model/resp/event/RespEventRdp.ts';

const CURSOR_IMAGE_HEADER_LENGTH = 9;

export interface RdpCursorImage {
    url: string;
    width: number;
    height: number;
    hotspotX: number;
    hotspotY: number;
}

export function useRdpCursor(canvasRef: RefObject<HTMLCanvasElement | null>) {
    const cursorUrlRef = useRef<string | null>(null);
    const [kind, setKind] = useState<number>(RDP_CURSOR_KIND.DEFAULT);
    const [image, setImage] = useState<RdpCursorImage | null>(null);

    const releaseCursorUrl = (): void => {
        const url = cursorUrlRef.current;
        if (!url) {
            return;
        }
        URL.revokeObjectURL(url);
        cursorUrlRef.current = null;
    };

    const applyCursorImage = (payload: ArrayBuffer): void => {
        const view = new DataView(payload);
        const hotspotX = view.getUint16(1, true);
        const hotspotY = view.getUint16(3, true);
        const width = view.getUint16(5, true);
        const height = view.getUint16(7, true);
        const pixels = new Uint8ClampedArray(payload, CURSOR_IMAGE_HEADER_LENGTH, width * height * 4);
        const source = document.createElement('canvas');
        source.width = width;
        source.height = height;
        const sourceContext = source.getContext('2d');
        if (!sourceContext) {
            throw new Error('RdpView cursor context unavailable');
        }
        sourceContext.putImageData(new ImageData(pixels, width, height), 0, 0);
        source.toBlob(blob => {
            if (!blob) {
                throw new Error('RdpView cursor encode failed');
            }
            const canvas = canvasRef.current;
            if (!canvas) {
                return;
            }
            releaseCursorUrl();
            const url = URL.createObjectURL(blob);
            cursorUrlRef.current = url;
            canvas.style.cursor = `url(${url}) ${hotspotX} ${hotspotY}, default`;
            setImage({url, width, height, hotspotX, hotspotY});
            setKind(RDP_CURSOR_KIND.IMAGE);
        });
    };

    const setCursor = (payload: ArrayBuffer): void => {
        const canvas = requireRdpCanvas(canvasRef.current);
        const cursorKind = new DataView(payload).getUint8(0);
        if (cursorKind === RDP_CURSOR_KIND.HIDDEN) {
            releaseCursorUrl();
            canvas.style.cursor = 'none';
            setImage(null);
            setKind(RDP_CURSOR_KIND.HIDDEN);
            return;
        }
        if (cursorKind === RDP_CURSOR_KIND.DEFAULT) {
            releaseCursorUrl();
            canvas.style.cursor = 'default';
            setImage(null);
            setKind(RDP_CURSOR_KIND.DEFAULT);
            return;
        }
        applyCursorImage(payload);
    };

    const dispose = (): void => {
        releaseCursorUrl();
    };

    return {kind, image, setCursor, dispose};
}
