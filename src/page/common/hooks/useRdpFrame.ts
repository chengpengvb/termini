import {type RefObject, useEffect, useRef} from 'react';
import {Avc444Renderer} from '@/page/common/util/avc444Renderer.ts';
import {RDP_FRAME_KIND_AVC444, RDP_FRAME_KIND_H264, RDP_FRAME_KIND_RAW, type RespRdpDesktopSize} from '@/common/model/resp/event/RespEventRdp.ts';
import {frameMime, h264ChunkType, h264CodecFromAnnexB, paintH264Frame, rdpFrameTimestamp, requireRdpCanvas, requireRdpContext} from '@/page/common/util/rdpUtil.ts';

const FRAME_RECT_HEADER_LENGTH = 13;

type H264Blit = {x: number; y: number; width: number; height: number; done: () => void};

export function useRdpFrame(
    canvasRef: RefObject<HTMLCanvasElement | null>,
    contextRef: RefObject<CanvasRenderingContext2D | null>,
) {
    const videoDecoderRef = useRef<VideoDecoder | null>(null);
    const videoCodecRef = useRef<string | null>(null);
    const avcRendererRef = useRef<Avc444Renderer | null>(null);
    const h264BlitsRef = useRef<H264Blit[]>([]);
    const latestFrameRef = useRef<VideoFrame | null>(null);
    const latestBlitRef = useRef<H264Blit | null>(null);
    const paintQueuedRef = useRef(false);
    const paintChainRef = useRef<Promise<void>>(Promise.resolve());
    const seenCodecsRef = useRef(new Set<number>());

    const enqueuePaint = (task: () => void | Promise<void>): void => {
        paintChainRef.current = paintChainRef.current.then(task).catch((error) => {
            console.error('[rdp] paint failed', error);
        });
    };

    const closeAvcRenderer = (): void => {
        avcRendererRef.current?.dispose();
        avcRendererRef.current = null;
    };

    const dropPendingH264 = (): void => {
        for (const blit of h264BlitsRef.current) {
            blit.done();
        }
        h264BlitsRef.current = [];
        latestFrameRef.current?.close();
        latestFrameRef.current = null;
        latestBlitRef.current?.done();
        latestBlitRef.current = null;
        paintQueuedRef.current = false;
    };

    const closeVideoDecoder = (): void => {
        const decoder = videoDecoderRef.current;
        if (decoder) {
            decoder.close();
        }
        videoDecoderRef.current = null;
        videoCodecRef.current = null;
        dropPendingH264();
    };

    const paintLatestH264 = (): void => {
        paintQueuedRef.current = false;
        const frame = latestFrameRef.current;
        const blit = latestBlitRef.current;
        latestFrameRef.current = null;
        latestBlitRef.current = null;
        if (!frame || !blit) {
            return;
        }
        try {
            paintH264Frame(requireRdpContext(contextRef.current), frame, blit.x, blit.y, blit.width, blit.height);
        } finally {
            frame.close();
            blit.done();
        }
    };

    const queueH264Paint = (frame: VideoFrame, blit: H264Blit): void => {
        latestFrameRef.current?.close();
        latestBlitRef.current?.done();
        latestFrameRef.current = frame;
        latestBlitRef.current = blit;
        if (paintQueuedRef.current) {
            return;
        }
        paintQueuedRef.current = true;
        requestAnimationFrame(paintLatestH264);
    };

    const resizeSurface = (size: RespRdpDesktopSize): void => {
        const canvas = requireRdpCanvas(canvasRef.current);
        if (canvas.width === size.width && canvas.height === size.height && contextRef.current) {
            return;
        }
        canvas.width = size.width;
        canvas.height = size.height;
        seenCodecsRef.current.clear();
        console.info('[rdp] surface', {width: size.width, height: size.height, dpr: window.devicePixelRatio, cssWidth: canvas.clientWidth, cssHeight: canvas.clientHeight});
        const context = canvas.getContext('2d', {alpha: false});
        if (!context) {
            throw new Error('RdpView 2d context unavailable');
        }
        contextRef.current = context;
        dropPendingH264();
        closeAvcRenderer();
    };

    const ensureVideoDecoder = (codec: string): VideoDecoder => {
        const existing = videoDecoderRef.current;
        if (existing && videoCodecRef.current === codec) {
            return existing;
        }
        const config: VideoDecoderConfig = {
            codec,
            optimizeForLatency: true,
            hardwareAcceleration: 'prefer-hardware',
        };
        if (existing) {
            existing.reset();
            existing.configure(config);
            videoCodecRef.current = codec;
            dropPendingH264();
            return existing;
        }
        const decoder = new VideoDecoder({
            output: (frame) => {
                const blit = h264BlitsRef.current.shift();
                if (!blit) {
                    frame.close();
                    return;
                }
                queueH264Paint(frame, blit);
            },
            error: (error) => {
                videoDecoderRef.current = null;
                videoCodecRef.current = null;
                dropPendingH264();
                console.error('[rdp] H.264 decoder failed', error);
            },
        });
        decoder.configure(config);
        videoDecoderRef.current = decoder;
        videoCodecRef.current = codec;
        return decoder;
    };

    const decodeH264Rect = (bytes: ArrayBuffer, timestampUs: number, x: number, y: number, width: number, height: number): void => {
        const data = new Uint8Array(bytes);
        const codec = h264CodecFromAnnexB(data);
        const existing = videoDecoderRef.current;
        if (!existing || (codec !== null && codec !== videoCodecRef.current)) {
            if (codec === null) {
                throw new Error('H.264 stream missing SPS');
            }
            if (h264ChunkType(data) !== 'key') {
                throw new Error('H.264 configure requires an IDR frame');
            }
        }
        const decoder = codec === null ? existing : ensureVideoDecoder(codec);
        if (!decoder) {
            throw new Error('H.264 decoder not configured');
        }
        let done!: () => void;
        const painted = new Promise<void>((resolve) => {
            done = resolve;
        });
        h264BlitsRef.current.push({x, y, width, height, done});
        decoder.decode(new EncodedVideoChunk({
            type: h264ChunkType(data),
            timestamp: timestampUs,
            data,
        }));
        enqueuePaint(() => painted);
    };

    const drawFrame = (payload: ArrayBuffer): Promise<number> => {
        const frameTs = rdpFrameTimestamp(payload);
        const view = new DataView(payload);
        let offset = 8;
        while (offset < payload.byteLength) {
            if (offset + FRAME_RECT_HEADER_LENGTH > payload.byteLength) {
                throw new Error(`RDP frame rect header truncated: offset=${offset} len=${payload.byteLength}`);
            }
            const kind = view.getUint8(offset);
            if (!seenCodecsRef.current.has(kind)) {
                seenCodecsRef.current.add(kind);
                console.info('[rdp] frame codec', kind === RDP_FRAME_KIND_H264 ? 'H264' : kind === RDP_FRAME_KIND_AVC444 ? 'AVC444' : kind === RDP_FRAME_KIND_RAW ? 'RGBA' : frameMime(kind));
            }
            const x = view.getUint16(offset + 1, true);
            const y = view.getUint16(offset + 3, true);
            const width = view.getUint16(offset + 5, true);
            const height = view.getUint16(offset + 7, true);
            const length = view.getUint32(offset + 9, true);
            offset += FRAME_RECT_HEADER_LENGTH;
            if (offset + length > payload.byteLength) {
                throw new Error(`RDP frame rect data truncated: need ${length} have ${payload.byteLength - offset}`);
            }
            const bytes = payload.slice(offset, offset + length);
            const canvas = requireRdpCanvas(canvasRef.current);
            const needWidth = x + width;
            const needHeight = y + height;
            if (canvas.width < needWidth || canvas.height < needHeight) {
                throw new Error(`RDP frame exceeds canvas: dest=${needWidth}x${needHeight} canvas=${canvas.width}x${canvas.height}`);
            }
            if (kind === RDP_FRAME_KIND_RAW) {
                enqueuePaint(() => {
                    requireRdpContext(contextRef.current).putImageData(new ImageData(new Uint8ClampedArray(bytes), width, height), x, y);
                });
            } else if (kind === RDP_FRAME_KIND_H264) {
                decodeH264Rect(bytes, frameTs * 1000, x, y, width, height);
            } else if (kind === RDP_FRAME_KIND_AVC444) {
                if (!avcRendererRef.current) {
                    avcRendererRef.current = new Avc444Renderer();
                }
                const renderer = avcRendererRef.current;
                const ticket = renderer.begin(new Uint8Array(bytes));
                enqueuePaint(() => renderer.present(requireRdpContext(contextRef.current), {x, y, width, height}, ticket));
            } else {
                const bitmap = createImageBitmap(new Blob([bytes], {type: frameMime(kind)}));
                enqueuePaint(async () => {
                    const image = await bitmap;
                    requireRdpContext(contextRef.current).drawImage(image, x, y, width, height);
                    image.close();
                });
            }
            offset += length;
        }
        return Promise.resolve(frameTs);
    };

    const dispose = (): void => {
        closeVideoDecoder();
        closeAvcRenderer();
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }
        const onLost = (event: Event) => {
            event.preventDefault();
            contextRef.current = null;
        };
        const onRestored = () => {
            const context = canvas.getContext('2d', {alpha: false});
            if (!context) {
                throw new Error('RdpView 2d context unavailable');
            }
            contextRef.current = context;
        };
        canvas.addEventListener('contextlost', onLost);
        canvas.addEventListener('contextrestored', onRestored);
        return () => {
            canvas.removeEventListener('contextlost', onLost);
            canvas.removeEventListener('contextrestored', onRestored);
        };
    }, [canvasRef, contextRef]);

    return {drawFrame, resizeSurface, dispose};
}
