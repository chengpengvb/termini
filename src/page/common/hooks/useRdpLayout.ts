import {type RefObject, useEffect, useRef} from 'react';
import {WindowSize} from '@/common/model/dto/WindowSize.ts';
import {focusRdpCanvas} from '@/page/common/util/rdpUtil.ts';
import {mobileRdpPixels, type RdpMobileQuality} from '@/page/common/util/rdpMobilePreferences.ts';

const ZOOM_LIMIT = 6;
const STANDARD_DPI = 96;
const RESIZE_MIN_DELAY = 200;
const DESKTOP_MAX_EDGE = 4096;

function deviceRatio(): number {
    const ratio = window.devicePixelRatio;
    if (!Number.isFinite(ratio) || ratio <= 0) {
        throw new Error('devicePixelRatio invalid');
    }
    return ratio;
}

function screenPixels(): { width: number; height: number } {
    const ratio = deviceRatio();
    return {
        width: Math.round(window.screen.width * ratio),
        height: Math.round(window.screen.height * ratio),
    };
}

export function mobileZoomMax(): number {
    return ZOOM_LIMIT;
}

export function useRdpLayout(
    hostRef: RefObject<HTMLDivElement | null>,
    canvasRef: RefObject<HTMLCanvasElement | null>,
    contextRef: RefObject<CanvasRenderingContext2D | null>,
    paneId: string,
    mobile: boolean,
    quality: RdpMobileQuality = 'balanced',
) {
    const qualityRef = useRef(quality);
    qualityRef.current = quality;
    const resizeTimerRef = useRef<number | null>(null);
    const initSizeObserverRef = useRef<ResizeObserver | null>(null);
    const cleanupResizeObserverRef = useRef<(() => void) | null>(null);
    const onResizeCallbackRef = useRef<((size: WindowSize) => void) | null>(null);
    const onVisibilityCallbackRef = useRef<((visible: boolean) => void) | null>(null);
    const lastReportedSizeRef = useRef<{ width: number; height: number } | null>(null);

    const mobileDesktopSize = (): WindowSize => {
        const screen = screenPixels();
        const pixels = mobileRdpPixels(screen.width, screen.height, qualityRef.current);
        return {
            cols: 0,
            rows: 0,
            dpi: STANDARD_DPI,
            pixel_width: pixels.width,
            pixel_height: pixels.height,
        };
    };

    const innerGetWindowSize = (host: HTMLDivElement): WindowSize => {
        if (mobile) {
            return mobileDesktopSize();
        }
        return {
            cols: 0,
            rows: 0,
            dpi: 0,
            pixel_width: Math.min(DESKTOP_MAX_EDGE, host.clientWidth),
            pixel_height: Math.min(DESKTOP_MAX_EDGE, host.clientHeight),
        };
    };

    const hostHasSize = (host: HTMLDivElement): boolean => {
        return host.clientWidth > 0 && host.clientHeight > 0;
    };

    const clearResizeTimer = (): void => {
        if (resizeTimerRef.current === null) {
            return;
        }
        window.clearTimeout(resizeTimerRef.current);
        resizeTimerRef.current = null;
    };

    const reportResize = (host: HTMLDivElement): void => {
        const size = innerGetWindowSize(host);
        if (size.pixel_width <= 0 || size.pixel_height <= 0) {
            return;
        }
        const callback = onResizeCallbackRef.current;
        if (!callback) {
            return;
        }
        const last = lastReportedSizeRef.current;
        if (last && last.width === size.pixel_width && last.height === size.pixel_height) {
            return;
        }
        lastReportedSizeRef.current = {width: size.pixel_width, height: size.pixel_height};
        callback(size);
    };

    const handleResize = (host: HTMLDivElement): void => {
        clearResizeTimer();
        resizeTimerRef.current = window.setTimeout(() => {
            resizeTimerRef.current = null;
            reportResize(host);
        }, RESIZE_MIN_DELAY);
    };

    const setupResizeObserver = (host: HTMLDivElement): void => {
        const resizeObserver = new ResizeObserver(() => handleResize(host));
        resizeObserver.observe(host);
        cleanupResizeObserverRef.current = () => {
            resizeObserver.disconnect();
            clearResizeTimer();
        };
    };

    const handleVisibilityChange = (): void => {
        onVisibilityCallbackRef.current?.(document.visibilityState === 'visible');
    };

    const bindVisibilityEvent = (): (() => void) => {
        document.addEventListener('visibilitychange', handleVisibilityChange);
        return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
    };

    const init = (onLayoutLoad: () => void): void => {
        const host = hostRef.current;
        const canvas = canvasRef.current;
        if (!host || !canvas) {
            return;
        }
        if (hostHasSize(host)) {
            canvas.width = host.clientWidth;
            canvas.height = host.clientHeight;
        }
        const context = canvas.getContext('2d', {alpha: false});
        if (!context) {
            throw new Error('RdpView 2d context unavailable');
        }
        contextRef.current = context;
        focusRdpCanvas(canvas);
        initSizeObserverRef.current?.disconnect();
        initSizeObserverRef.current = null;
        if (hostHasSize(host)) {
            onLayoutLoad();
            return;
        }
        const observer = new ResizeObserver(() => {
            if (!hostHasSize(host)) {
                return;
            }
            observer.disconnect();
            if (initSizeObserverRef.current === observer) {
                initSizeObserverRef.current = null;
            }
            onLayoutLoad();
        });
        initSizeObserverRef.current = observer;
        observer.observe(host);
    };

    const onWindowResize = (cb: (size: WindowSize) => void): (() => void) => {
        onResizeCallbackRef.current = cb;
        return () => {
            if (onResizeCallbackRef.current === cb) {
                onResizeCallbackRef.current = null;
            }
        };
    };

    const onVisibilityChange = (cb: (visible: boolean) => void): (() => void) => {
        onVisibilityCallbackRef.current = cb;
        return () => {
            if (onVisibilityCallbackRef.current === cb) {
                onVisibilityCallbackRef.current = null;
            }
        };
    };

    const getWindowSize = (): WindowSize => {
        const host = hostRef.current;
        if (!host) {
            throw new Error('RdpView is not initialized');
        }
        return innerGetWindowSize(host);
    };

    const dispose = (): void => {
        initSizeObserverRef.current?.disconnect();
        initSizeObserverRef.current = null;
        lastReportedSizeRef.current = null;
    };

    useEffect(() => {
        const host = hostRef.current;
        if (!host) {
            return;
        }
        setupResizeObserver(host);
        const unbindVisibility = bindVisibilityEvent();
        return () => {
            unbindVisibility();
            cleanupResizeObserverRef.current?.();
            cleanupResizeObserverRef.current = null;
        };
    }, [paneId]);

    useEffect(() => {
        const host = hostRef.current;
        if (mobile && host && hostHasSize(host)) {
            reportResize(host);
        }
    }, [quality]);

    return {init, getWindowSize, onWindowResize, onVisibilityChange, dispose};
}
