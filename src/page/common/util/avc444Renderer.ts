import { h264ChunkType, h264CodecFromAnnexB } from '@/page/common/util/rdpUtil.ts';

const AVC_FLAG_V2 = 0x01;
const AVC_FLAG_444 = 0x02;
const FRAME_DURATION_US = 16667;

const VERTEX_SHADER = `#version 300 es
const vec2 POSITIONS[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
void main() {
    gl_Position = vec4(POSITIONS[gl_VertexID], 0.0, 1.0);
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision highp float;
precision highp int;
uniform sampler2D uMainY;
uniform sampler2D uMainU;
uniform sampler2D uMainV;
uniform sampler2D uAuxY;
uniform sampler2D uAuxU;
uniform sampler2D uAuxV;
uniform int uMode;
uniform int uHeight;
uniform int uAuxWidth;
out vec4 outColor;

void main() {
    int x = int(gl_FragCoord.x);
    int y = uHeight - 1 - int(gl_FragCoord.y);
    ivec2 full = ivec2(x, y);
    ivec2 half_ = ivec2(x >> 1, y >> 1);
    float lumaY = texelFetch(uMainY, full, 0).r;
    float u = texelFetch(uMainU, half_, 0).r;
    float v = texelFetch(uMainV, half_, 0).r;
    if (uMode == 2) {
        if ((x & 1) == 1) {
            u = texelFetch(uAuxY, ivec2(x >> 1, y), 0).r;
            v = texelFetch(uAuxY, ivec2((x >> 1) + uAuxWidth / 2, y), 0).r;
        } else if ((y & 1) == 1) {
            ivec2 quarter = ivec2(x >> 2, y >> 1);
            if ((x & 3) == 0) {
                u = texelFetch(uAuxU, quarter, 0).r;
                v = texelFetch(uAuxU, ivec2(quarter.x + uAuxWidth / 4, quarter.y), 0).r;
            } else {
                u = texelFetch(uAuxV, quarter, 0).r;
                v = texelFetch(uAuxV, ivec2(quarter.x + uAuxWidth / 4, quarter.y), 0).r;
            }
        }
    } else if (uMode == 1) {
        if ((y & 1) == 1) {
            int odd = y >> 1;
            int auxRow = (odd >> 3) * 16 + (odd & 7);
            u = texelFetch(uAuxY, ivec2(x, auxRow), 0).r;
            v = texelFetch(uAuxY, ivec2(x, auxRow + 8), 0).r;
        } else if ((x & 1) == 1) {
            u = texelFetch(uAuxU, half_, 0).r;
            v = texelFetch(uAuxV, half_, 0).r;
        }
    }
    float luma = (lumaY - 0.0627451) * 1.1643836;
    float cb = u - 0.5019608;
    float cr = v - 0.5019608;
    outColor = vec4(
        luma + 1.5960268 * cr,
        luma - 0.3917623 * cb - 0.8129676 * cr,
        luma + 2.0172321 * cb,
        1.0
    );
}
`;

type AvcRect = { x: number; y: number; width: number; height: number };

type AvcSegment = { rects: AvcRect[]; data: Uint8Array };

type AvcPayload = { mode: number; luma: AvcSegment | null; chroma: AvcSegment | null };

type AvcStreamDecoder = {
    decoder: VideoDecoder | null;
    codec: string | null;
    timestamp: number;
    waiters: Array<{ resolve: (frame: VideoFrame) => void; reject: (error: unknown) => void }>;
};

type PlaneData = { data: Uint8Array; stride: number };

type FramePlanes = { y: PlaneData; u: PlaneData; v: PlaneData; width: number; height: number };

type AvcDraw = {
    parsed: AvcPayload;
    luma: Promise<VideoFrame | null> | null;
    chroma: Promise<VideoFrame | null> | null;
    discarded: boolean;
};

const newStreamDecoder = (): AvcStreamDecoder => ({ decoder: null, codec: null, timestamp: 0, waiters: [] });

const readSegment = (view: DataView, bytes: Uint8Array, start: number): { segment: AvcSegment | null; next: number } => {
    if (start + 2 > bytes.length) {
        throw new Error(`AVC segment header truncated: offset=${start} len=${bytes.length}`);
    }
    const count = view.getUint16(start, true);
    let offset = start + 2;
    if (offset + count * 8 + 4 > bytes.length) {
        throw new Error(`AVC segment metadata truncated: rects=${count} offset=${offset} len=${bytes.length}`);
    }
    const rects: AvcRect[] = [];
    for (let index = 0; index < count; index += 1) {
        rects.push({
            x: view.getUint16(offset, true),
            y: view.getUint16(offset + 2, true),
            width: view.getUint16(offset + 4, true),
            height: view.getUint16(offset + 6, true),
        });
        offset += 8;
    }
    const length = view.getUint32(offset, true);
    offset += 4;
    if (offset + length > bytes.length) {
        throw new Error(`AVC segment bitstream truncated: need=${length} have=${bytes.length - offset}`);
    }
    if (length === 0) {
        return { segment: null, next: offset };
    }
    const data = bytes.subarray(offset, offset + length);
    return { segment: { rects, data }, next: offset + length };
};

const parseAvcPayload = (bytes: Uint8Array): AvcPayload => {
    if (bytes.length < 1) {
        throw new Error('AVC payload is empty');
    }
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const flags = bytes[0];
    const lumaPart = readSegment(view, bytes, 1);
    const chromaPart = readSegment(view, bytes, lumaPart.next);
    if (chromaPart.next !== bytes.length) {
        throw new Error(`AVC payload has trailing bytes: end=${chromaPart.next} len=${bytes.length}`);
    }
    const mode = (flags & AVC_FLAG_444) === 0 ? 0 : (flags & AVC_FLAG_V2) === 0 ? 1 : 2;
    return { mode, luma: lumaPart.segment, chroma: chromaPart.segment };
};

const rgbaToPlanes = (buffer: Uint8Array, stride: number, width: number, height: number, bgra: boolean): FramePlanes => {
    const y = new Uint8Array(width * height);
    const halfWidth = width >> 1;
    const halfHeight = height >> 1;
    const u = new Uint8Array(halfWidth * halfHeight);
    const v = new Uint8Array(halfWidth * halfHeight);
    for (let row = 0; row < height; row += 1) {
        const srcRow = row * stride;
        const yRow = row * width;
        for (let col = 0; col < width; col += 1) {
            const i = srcRow + col * 4;
            const r = bgra ? buffer[i + 2] : buffer[i];
            const g = buffer[i + 1];
            const b = bgra ? buffer[i] : buffer[i + 2];
            y[yRow + col] = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
        }
    }
    for (let row = 0; row < halfHeight; row += 1) {
        const srcRow = row * 2 * stride;
        const srcRow2 = srcRow + stride;
        const uvRow = row * halfWidth;
        for (let col = 0; col < halfWidth; col += 1) {
            const c = col * 8;
            const i0 = srcRow + c;
            const i1 = srcRow + c + 4;
            const i2 = srcRow2 + c;
            const i3 = srcRow2 + c + 4;
            const r = ((bgra ? buffer[i0 + 2] : buffer[i0]) + (bgra ? buffer[i1 + 2] : buffer[i1]) + (bgra ? buffer[i2 + 2] : buffer[i2]) + (bgra ? buffer[i3 + 2] : buffer[i3])) >> 2;
            const g = (buffer[i0 + 1] + buffer[i1 + 1] + buffer[i2 + 1] + buffer[i3 + 1]) >> 2;
            const b = ((bgra ? buffer[i0] : buffer[i0 + 2]) + (bgra ? buffer[i1] : buffer[i1 + 2]) + (bgra ? buffer[i2] : buffer[i2 + 2]) + (bgra ? buffer[i3] : buffer[i3 + 2])) >> 2;
            u[uvRow + col] = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
            v[uvRow + col] = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
        }
    }
    return {
        width,
        height,
        y: { data: y, stride: width },
        u: { data: u, stride: halfWidth },
        v: { data: v, stride: halfWidth },
    };
};

const extractPlanes = async (frame: VideoFrame): Promise<FramePlanes> => {
    const width = frame.codedWidth;
    const height = frame.codedHeight;
    const rect = { x: 0, y: 0, width, height };
    const buffer = new Uint8Array(frame.allocationSize({ rect }));
    const layout = await frame.copyTo(buffer, { rect });
    const halfWidth = width >> 1;
    const halfHeight = height >> 1;
    if (frame.format === 'I420') {
        return {
            width,
            height,
            y: { data: buffer.subarray(layout[0].offset), stride: layout[0].stride },
            u: { data: buffer.subarray(layout[1].offset), stride: layout[1].stride },
            v: { data: buffer.subarray(layout[2].offset), stride: layout[2].stride },
        };
    }
    if (frame.format === 'NV12') {
        const u = new Uint8Array(halfWidth * halfHeight);
        const v = new Uint8Array(halfWidth * halfHeight);
        const uvStride = layout[1].stride;
        for (let row = 0; row < halfHeight; row += 1) {
            const source = layout[1].offset + row * uvStride;
            const target = row * halfWidth;
            for (let column = 0; column < halfWidth; column += 1) {
                u[target + column] = buffer[source + column * 2];
                v[target + column] = buffer[source + column * 2 + 1];
            }
        }
        return {
            width,
            height,
            y: { data: buffer.subarray(layout[0].offset), stride: layout[0].stride },
            u: { data: u, stride: halfWidth },
            v: { data: v, stride: halfWidth },
        };
    }
    if (frame.format === 'RGBA' || frame.format === 'RGBX') {
        return rgbaToPlanes(buffer, layout[0].stride, width, height, false);
    }
    if (frame.format === 'BGRA' || frame.format === 'BGRX') {
        return rgbaToPlanes(buffer, layout[0].stride, width, height, true);
    }
    throw new Error(`AVC frame format unsupported: ${frame.format}`);
};

export class Avc444Renderer {
    private nextTicket = 0;
    private auxWidth = 0;
    private auxValid = false;
    private lumaValid = false;
    private readonly textures: WebGLTexture[];
    private readonly canvas: HTMLCanvasElement;
    private readonly gl: WebGL2RenderingContext;
    private readonly stream = newStreamDecoder();
    private readonly draws = new Map<number, AvcDraw>();
    private readonly modeLocation: WebGLUniformLocation;
    private readonly heightLocation: WebGLUniformLocation;
    private readonly auxWidthLocation: WebGLUniformLocation;

    constructor() {
        this.canvas = document.createElement('canvas');
        const gl = this.canvas.getContext('webgl2');
        if (!gl) {
            throw new Error('AVC444 requires WebGL2');
        }
        this.gl = gl;
        const program = gl.createProgram();
        gl.attachShader(program, this.compileShader(gl.VERTEX_SHADER, VERTEX_SHADER));
        gl.attachShader(program, this.compileShader(gl.FRAGMENT_SHADER, FRAGMENT_SHADER));
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            throw new Error(`AVC444 shader link failed: ${gl.getProgramInfoLog(program)}`);
        }
        gl.useProgram(program);
        const samplers = ['uMainY', 'uMainU', 'uMainV', 'uAuxY', 'uAuxU', 'uAuxV'];
        this.textures = samplers.map((name, unit) => {
            const texture = gl.createTexture();
            gl.activeTexture(gl.TEXTURE0 + unit);
            gl.bindTexture(gl.TEXTURE_2D, texture);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
            const location = gl.getUniformLocation(program, name);
            if (!location) {
                throw new Error(`AVC444 sampler missing: ${name}`);
            }
            gl.uniform1i(location, unit);
            return texture;
        });
        this.modeLocation = this.requireUniform(program, 'uMode');
        this.heightLocation = this.requireUniform(program, 'uHeight');
        this.auxWidthLocation = this.requireUniform(program, 'uAuxWidth');
        gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    }

    private requireUniform(program: WebGLProgram, name: string): WebGLUniformLocation {
        const location = this.gl.getUniformLocation(program, name);
        if (!location) {
            throw new Error(`AVC444 uniform missing: ${name}`);
        }
        return location;
    }

    private compileShader(type: number, source: string): WebGLShader {
        const gl = this.gl;
        const shader = gl.createShader(type);
        if (!shader) {
            throw new Error('AVC444 shader creation failed');
        }
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            throw new Error(`AVC444 shader compile failed: ${gl.getShaderInfoLog(shader)}`);
        }
        return shader;
    }

    private configureDecoder(slot: AvcStreamDecoder, codec: string): void {
        const config: VideoDecoderConfig = { codec, optimizeForLatency: true, hardwareAcceleration: 'prefer-hardware' };
        if (slot.decoder) {
            const waiters = slot.waiters.splice(0);
            for (const waiter of waiters) {
                waiter.reject(new Error('AVC decoder reconfigured'));
            }
            slot.decoder.reset();
            slot.decoder.configure(config);
            slot.codec = codec;
            return;
        }
        const decoder = new VideoDecoder({
            output: (frame) => {
                const waiter = slot.waiters.shift();
                if (!waiter) {
                    frame.close();
                    throw new Error('AVC output has no matching input');
                }
                waiter.resolve(frame);
            },
            error: (error) => {
                slot.decoder = null;
                slot.codec = null;
                const waiters = slot.waiters.splice(0);
                for (const waiter of waiters) {
                    waiter.reject(error);
                }
            },
        });
        decoder.configure(config);
        slot.decoder = decoder;
        slot.codec = codec;
    }

    private decodeStream(slot: AvcStreamDecoder, bytes: Uint8Array): Promise<VideoFrame> {
        return new Promise((resolve, reject) => {
            const codec = h264CodecFromAnnexB(bytes);
            if (!slot.decoder || (codec !== null && codec !== slot.codec)) {
                if (codec === null) {
                    throw new Error('AVC stream missing SPS');
                }
                if (h264ChunkType(bytes) !== 'key') {
                    throw new Error('AVC configure requires an IDR frame');
                }
                this.configureDecoder(slot, codec);
            }
            if (!slot.decoder) {
                throw new Error('AVC decoder not configured');
            }
            slot.waiters.push({ resolve, reject });
            slot.decoder.decode(new EncodedVideoChunk({ type: h264ChunkType(bytes), timestamp: slot.timestamp, data: bytes }));
            slot.timestamp += FRAME_DURATION_US;
        });
    }

    private uploadPlane(index: number, plane: PlaneData, width: number, height: number): void {
        const gl = this.gl;
        gl.activeTexture(gl.TEXTURE0 + index);
        gl.bindTexture(gl.TEXTURE_2D, this.textures[index]);
        gl.pixelStorei(gl.UNPACK_ROW_LENGTH, plane.stride);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, plane.data);
        gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    }

    private uploadFrame(planes: FramePlanes, main: boolean): void {
        const base = main ? 0 : 3;
        this.uploadPlane(base, planes.y, planes.width, planes.height);
        this.uploadPlane(base + 1, planes.u, planes.width >> 1, planes.height >> 1);
        this.uploadPlane(base + 2, planes.v, planes.width >> 1, planes.height >> 1);
        if (main) {
            if (this.canvas.width !== planes.width || this.canvas.height !== planes.height) {
                this.canvas.width = planes.width;
                this.canvas.height = planes.height;
            }
            this.lumaValid = true;
        } else {
            this.auxWidth = planes.width;
            this.auxValid = true;
        }
    }

    begin(payload: Uint8Array): number {
        const ticket = ++this.nextTicket;
        const parsed = parseAvcPayload(payload);
        const draw: AvcDraw = {
            parsed,
            luma: parsed.luma ? this.decodeStream(this.stream, parsed.luma.data) : null,
            chroma: parsed.chroma ? this.decodeStream(this.stream, parsed.chroma.data) : null,
            discarded: false,
        };
        this.draws.set(ticket, draw);
        return ticket;
    }

    async present(context: CanvasRenderingContext2D, dest: AvcRect, ticket: number): Promise<void> {
        const draw = this.draws.get(ticket);
        if (!draw) {
            return;
        }
        const luma = draw.luma ? await draw.luma : null;
        const chroma = draw.chroma ? await draw.chroma : null;
        if (draw.discarded) {
            luma?.close();
            chroma?.close();
            return;
        }
        try {
            if (luma) {
                const planes = await extractPlanes(luma);
                if (draw.discarded) {
                    return;
                }
                this.uploadFrame(planes, true);
            }
            if (chroma) {
                const planes = await extractPlanes(chroma);
                if (draw.discarded) {
                    return;
                }
                this.uploadFrame(planes, false);
            }
            if (!this.lumaValid) {
                throw new Error('AVC main stream missing');
            }
            this.paint(context, dest, draw.parsed);
        } finally {
            this.discard(draw, ticket, luma, chroma);
        }
    }

    private discard(draw: AvcDraw, ticket: number, luma: VideoFrame | null, chroma: VideoFrame | null): void {
        if (draw.discarded) {
            return;
        }
        draw.discarded = true;
        this.draws.delete(ticket);
        luma?.close();
        chroma?.close();
    }

    private paint(context: CanvasRenderingContext2D, dest: AvcRect, parsed: AvcPayload): void {
        const mode = this.auxValid ? parsed.mode : 0;
        const gl = this.gl;
        gl.viewport(0, 0, this.canvas.width, this.canvas.height);
        gl.uniform1i(this.modeLocation, mode);
        gl.uniform1i(this.heightLocation, this.canvas.height);
        gl.uniform1i(this.auxWidthLocation, this.auxWidth);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        const rects = [...(parsed.luma?.rects ?? []), ...(parsed.chroma?.rects ?? [])];
        for (const rect of rects) {
            context.drawImage(this.canvas, rect.x, rect.y, rect.width, rect.height, dest.x + rect.x, dest.y + rect.y, rect.width, rect.height);
        }
    }

    dispose(): void {
        for (const [ticket, draw] of this.draws) {
            draw.discarded = true;
            this.draws.delete(ticket);
            void Promise.all([draw.luma ?? Promise.resolve(null), draw.chroma ?? Promise.resolve(null)])
                .then(([luma, chroma]) => {
                    luma?.close();
                    chroma?.close();
                })
                .catch(() => undefined);
        }
        const slot = this.stream;
        const waiters = slot.waiters.splice(0);
        for (const waiter of waiters) {
            waiter.reject(new Error('AVC renderer destroyed'));
        }
        slot.decoder?.close();
        slot.decoder = null;
        slot.codec = null;
        const extension = this.gl.getExtension('WEBGL_lose_context');
        extension?.loseContext();
    }
}
