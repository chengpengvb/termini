import {useEffect, useRef} from 'react';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {dismissTopBack, installBackDismissHistoryTrap} from '@/page/common/util/backDismissRegistry.ts';

const AXIS_VERTICAL = 0;
const AXIS_HORIZONTAL = 1;
const EDGE_ZONE_PX = 24;
const AXIS_LOCK_PX = 8;
const COMMIT_DX_PX = 48;
const CUE_MAX_PULL = 72;
const COMMIT_PROGRESS = 0.55;
const VELOCITY_PX_MS = 0.45;
const KEYBOARD_OPEN_INSET_PX = 80;
const KEYBOARD_CLOSE_SETTLE_MS = 240;

declare global {
    interface Window {
        __tmConsumeBack?: () => boolean;
        TerminiNative?: {
            exitApp: () => void;
            showSoftInput: () => void;
            hideSoftInput: () => void;
            setScreenOrientation: (lock: string) => void;
        };
    }
}

function isGestureIgnored(target: EventTarget | null): boolean {
    if (!(target instanceof Element)) {
        return false;
    }
    return target.closest('[data-tm-gesture-ignore="true"]') != null;
}

function exitAndroidApp(): void {
    if (window.TerminiNative) {
        window.TerminiNative.exitApp();
        return;
    }
    throw new Error('TerminiNative.exitApp unavailable');
}

function resolveProgress(dx: number): number {
    if (dx <= 0) {
        return 0;
    }
    return Math.min(1, dx / CUE_MAX_PULL);
}

function scrollFocusedIntoOverflowAncestor(target: HTMLElement): void {
    if (!target.isConnected) {
        return;
    }
    let current = target.parentElement;
    while (current && current !== document.documentElement) {
        const overflowY = getComputedStyle(current).overflowY;
        if ((overflowY === 'auto' || overflowY === 'scroll') && current.scrollHeight > current.clientHeight) {
            const parentRect = current.getBoundingClientRect();
            const targetRect = target.getBoundingClientRect();
            if (targetRect.bottom > parentRect.bottom) {
                current.scrollTop += targetRect.bottom - parentRect.bottom;
            } else if (targetRect.top < parentRect.top) {
                current.scrollTop -= parentRect.top - targetRect.top;
            }
            return;
        }
        current = current.parentElement;
    }
}

export function MobileGestureHost() {
    const mobile = usePlatformMobile();
    const systemUtilService = useSystemUtilService();
    const androidRef = useRef(false);
    const trackingRef = useRef<{
        pointerId: number;
        startX: number;
        startY: number;
        startT: number;
        axis: number | null;
        edgeArmed: boolean;
    } | null>(null);

    useEffect(() => {
        let cancelled = false;
        void systemUtilService.getPlatformInfo().then((info) => {
            if (!cancelled) {
                androidRef.current = info.os === 'android';
            }
        });
        return () => {
            cancelled = true;
        };
    }, [systemUtilService]);

    useEffect(() => {
        if (!mobile) {
            return;
        }
        return installBackDismissHistoryTrap();
    }, [mobile]);

    useEffect(() => {
        if (!mobile) {
            return;
        }

        let keyboardCloseTimer = 0;

        const readKeyboardInset = (): number => {
            const root = document.documentElement;
            const vv = window.visualViewport;
            const height = vv ? vv.height : window.innerHeight;
            const imeRaw = getComputedStyle(root).getPropertyValue('--tm-ime-inset').trim();
            const nativeInset = Number.parseFloat(imeRaw);
            if (imeRaw.length > 0 && Number.isFinite(nativeInset)) {
                return Math.max(0, nativeInset);
            }
            return Math.max(0, window.innerHeight - height);
        };

        const syncKeyboardClass = (keyboardInset: number): void => {
            const root = document.documentElement;
            if (keyboardInset > KEYBOARD_OPEN_INSET_PX) {
                window.clearTimeout(keyboardCloseTimer);
                keyboardCloseTimer = 0;
                root.classList.add('tm-keyboard-open');
                return;
            }
            if (!root.classList.contains('tm-keyboard-open') || keyboardCloseTimer !== 0) {
                return;
            }
            keyboardCloseTimer = window.setTimeout(() => {
                keyboardCloseTimer = 0;
                if (readKeyboardInset() <= KEYBOARD_OPEN_INSET_PX) {
                    root.classList.remove('tm-keyboard-open');
                }
            }, KEYBOARD_CLOSE_SETTLE_MS);
        };

        const syncViewportVars = (): void => {
            const root = document.documentElement;
            const vv = window.visualViewport;
            const height = vv ? vv.height : window.innerHeight;
            const offsetTop = vv ? vv.offsetTop : 0;
            const keyboardInset = readKeyboardInset();
            root.style.setProperty('--tm-vv-height', `${height}px`);
            root.style.setProperty('--tm-vv-offset-top', `${offsetTop}px`);
            root.style.setProperty('--tm-keyboard-inset', `${keyboardInset}px`);
            root.style.setProperty('--tm-app-height', `${height}px`);
            syncKeyboardClass(keyboardInset);
        };

        syncViewportVars();
        const vv = window.visualViewport;
        vv?.addEventListener('resize', syncViewportVars);
        vv?.addEventListener('scroll', syncViewportVars);
        window.addEventListener('resize', syncViewportVars);
        document.addEventListener('focusin', syncViewportVars);
        document.addEventListener('focusout', syncViewportVars);
        return () => {
            window.clearTimeout(keyboardCloseTimer);
            vv?.removeEventListener('resize', syncViewportVars);
            vv?.removeEventListener('scroll', syncViewportVars);
            window.removeEventListener('resize', syncViewportVars);
            document.removeEventListener('focusin', syncViewportVars);
            document.removeEventListener('focusout', syncViewportVars);
            const root = document.documentElement;
            root.style.removeProperty('--tm-vv-height');
            root.style.removeProperty('--tm-vv-offset-top');
            root.style.removeProperty('--tm-keyboard-inset');
            root.style.removeProperty('--tm-app-height');
            root.classList.remove('tm-keyboard-open');
        };
    }, [mobile]);

    useEffect(() => {
        if (!mobile) {
            return;
        }

        let focusScrollTimer = 0;
        const onFocusIn = (event: FocusEvent): void => {
            const target = event.target;
            if (!(target instanceof HTMLElement)) {
                return;
            }
            if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA' && !target.isContentEditable) {
                return;
            }
            window.clearTimeout(focusScrollTimer);
            focusScrollTimer = window.setTimeout(() => {
                scrollFocusedIntoOverflowAncestor(target);
            }, 80);
        };

        document.addEventListener('focusin', onFocusIn);
        return () => {
            window.clearTimeout(focusScrollTimer);
            document.removeEventListener('focusin', onFocusIn);
        };
    }, [mobile]);

    useEffect(() => {
        if (!mobile) {
            window.__tmConsumeBack = undefined;
            return;
        }

        window.__tmConsumeBack = (): boolean => {
            return dismissTopBack();
        };

        return () => {
            window.__tmConsumeBack = undefined;
        };
    }, [mobile]);

    useEffect(() => {
        if (!mobile) {
            return;
        }

        const onPointerDown = (event: PointerEvent): void => {
            if (event.pointerType === 'mouse' && event.button !== 0) {
                return;
            }
            if (event.clientX > EDGE_ZONE_PX) {
                return;
            }
            if (isGestureIgnored(event.target)) {
                return;
            }
            trackingRef.current = {
                pointerId: event.pointerId,
                startX: event.clientX,
                startY: event.clientY,
                startT: performance.now(),
                axis: null,
                edgeArmed: true,
            };
        };

        const onPointerMove = (event: PointerEvent): void => {
            const tracking = trackingRef.current;
            if (!tracking || tracking.pointerId !== event.pointerId || !tracking.edgeArmed) {
                return;
            }
            const dx = event.clientX - tracking.startX;
            const dy = event.clientY - tracking.startY;
            if (tracking.axis === null) {
                if (Math.abs(dx) < AXIS_LOCK_PX && Math.abs(dy) < AXIS_LOCK_PX) {
                    return;
                }
                tracking.axis = Math.abs(dx) > Math.abs(dy) ? AXIS_HORIZONTAL : AXIS_VERTICAL;
                if (tracking.axis !== AXIS_HORIZONTAL) {
                    trackingRef.current = null;
                    return;
                }
            }
            if (tracking.axis === AXIS_HORIZONTAL && event.cancelable) {
                event.preventDefault();
            }
        };

        const onTouchMove = (event: TouchEvent): void => {
            const tracking = trackingRef.current;
            if (!tracking || !tracking.edgeArmed || tracking.axis !== AXIS_HORIZONTAL) {
                return;
            }
            if (event.cancelable) {
                event.preventDefault();
            }
        };

        const finish = (event: PointerEvent): void => {
            const tracking = trackingRef.current;
            if (!tracking || tracking.pointerId !== event.pointerId) {
                return;
            }
            trackingRef.current = null;
            const dx = event.clientX - tracking.startX;
            const progress = resolveProgress(dx);
            if (tracking.axis !== AXIS_HORIZONTAL) {
                return;
            }
            if (dx <= 0) {
                return;
            }
            const elapsed = Math.max(1, performance.now() - tracking.startT);
            const velocity = dx / elapsed;
            if (progress < COMMIT_PROGRESS && dx < COMMIT_DX_PX && velocity < VELOCITY_PX_MS) {
                return;
            }
            if (dismissTopBack()) {
                return;
            }
            if (androidRef.current) {
                exitAndroidApp();
            }
        };

        window.addEventListener('pointerdown', onPointerDown, {capture: true, passive: true});
        window.addEventListener('pointermove', onPointerMove, {capture: true, passive: false});
        window.addEventListener('touchmove', onTouchMove, {capture: true, passive: false});
        window.addEventListener('pointerup', finish, {capture: true, passive: true});
        window.addEventListener('pointercancel', finish, {capture: true, passive: true});
        return () => {
            window.removeEventListener('pointerdown', onPointerDown, true);
            window.removeEventListener('pointermove', onPointerMove, true);
            window.removeEventListener('touchmove', onTouchMove, true);
            window.removeEventListener('pointerup', finish, true);
            window.removeEventListener('pointercancel', finish, true);
        };
    }, [mobile]);

    return null;
}
