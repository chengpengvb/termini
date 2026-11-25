import {type RefObject, useLayoutEffect, useRef, useState} from 'react';

// RDP has an explicit keyboard button. A focused/stale textarea is not consent
// to show the keyboard, unlike a local terminal's tap-to-type interaction.
export function useRdpSoftKeyboard(
    canvasRef: RefObject<HTMLCanvasElement | null>,
    imeRef: RefObject<HTMLTextAreaElement | null>,
    mobile: boolean,
    visible: boolean,
    request: (show: boolean) => void,
) {
    const allowedRef = useRef(false);
    const seenVisibleRef = useRef(false);
    const buttonFocusRef = useRef(false);
    const requestRef = useRef(request);
    const [requested, setRequested] = useState(false);
    requestRef.current = request;

    const lock = (): void => {
        const el = imeRef.current;
        allowedRef.current = false;
        seenVisibleRef.current = false;
        if (el) {
            el.readOnly = true;
            el.inputMode = 'none';
            if (document.activeElement === el) {
                el.blur();
                canvasRef.current?.focus({preventScroll: true});
            }
        }
        setRequested(false);
    };

    const open = (): void => {
        const el = imeRef.current;
        if (!mobile || !el) {
            return;
        }
        allowedRef.current = true;
        seenVisibleRef.current = visible;
        // Update synchronously inside the button's user gesture, before React's
        // next render; iOS will reject a deferred programmatic keyboard request.
        el.readOnly = false;
        el.inputMode = 'text';
        setRequested(true);
        buttonFocusRef.current = true;
        try {
            el.focus({preventScroll: true});
        } finally {
            buttonFocusRef.current = false;
        }
        requestRef.current(true);
    };

    const hide = (): void => {
        lock();
        requestRef.current(false);
    };

    useLayoutEffect(() => {
        if (!mobile) {
            lock();
            return;
        }
        if (allowedRef.current && visible) {
            seenVisibleRef.current = true;
        } else if (allowedRef.current && seenVisibleRef.current) {
            // System Back / keyboard's own hide button revokes the request too.
            lock();
        } else if (!allowedRef.current && visible && canvasRef.current?.parentElement?.contains(document.activeElement)) {
            // Reject a stale native input connection reopening this session.
            requestRef.current(false);
        }
    }, [mobile, visible]);

    useLayoutEffect(() => {
        const el = imeRef.current;
        const host = canvasRef.current?.parentElement;
        if (!el || !host) {
            return;
        }
        el.readOnly = true;
        el.inputMode = 'none';
        const guardFocus = (): void => {
            if (!allowedRef.current || !buttonFocusRef.current) {
                lock();
            }
        };
        const preserve = (event: PointerEvent): void => {
            if (allowedRef.current && document.activeElement === el) {
                event.preventDefault();
            }
        };
        const hidden = (): void => {
            if (document.hidden) {
                lock();
            }
        };
        el.addEventListener('focus', guardFocus);
        host.addEventListener('pointerdown', preserve, {capture: true, passive: false});
        document.addEventListener('visibilitychange', hidden);
        return () => {
            allowedRef.current = false;
            el.readOnly = true;
            el.inputMode = 'none';
            el.removeEventListener('focus', guardFocus);
            host.removeEventListener('pointerdown', preserve, true);
            document.removeEventListener('visibilitychange', hidden);
        };
    }, [canvasRef, imeRef]);

    return {open, hide, requested};
}
