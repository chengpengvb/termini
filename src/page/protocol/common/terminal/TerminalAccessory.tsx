import {Box, ButtonBase} from '@mui/material';
import {KeyboardHide} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useImperativeHandle, useRef, useState, type PointerEvent as ReactPointerEvent, type Ref} from 'react';

export interface TerminalAccessoryHandle {
    consume: (data: string) => string;
}

interface TerminalAccessoryProps {
    onHide: () => void;
    onOpenSearch: () => void;
    onOpenSuggest: () => void;
    onInput: (data: string) => void;
    ref?: Ref<TerminalAccessoryHandle>;
}

interface Sticky {
    active: boolean;
    locked: boolean;
}

const MOD_ALT = 1;
const MOD_CTRL = 0;
const MOD_SHIFT = 2;
const REPEAT_MS = 80;
const LONG_PRESS_MS = 400;
const SEQ = {
    ESC: '\x1b',
    TAB: '\t',
    BKSP: '\x7f',
    DEL: '\x1b[3~',
    SPACE: ' ',
    UP: '\x1b[A',
    DOWN: '\x1b[B',
    RIGHT: '\x1b[C',
    LEFT: '\x1b[D',
    HOME: '\x1b[H',
    END: '\x1b[F',
    PGUP: '\x1b[5~',
    PGDN: '\x1b[6~',
} as const;

const CSI: Record<string, readonly [string, string]> = {
    '\x1b[A': ['\x1b[1', 'A'],
    '\x1b[B': ['\x1b[1', 'B'],
    '\x1b[C': ['\x1b[1', 'C'],
    '\x1b[D': ['\x1b[1', 'D'],
    '\x1b[H': ['\x1b[1', 'H'],
    '\x1b[F': ['\x1b[1', 'F'],
    '\x1b[5~': ['\x1b[5', '~'],
    '\x1b[6~': ['\x1b[6', '~'],
    '\x1b[3~': ['\x1b[3', '~'],
    '\x1bOP': ['\x1b[1', 'P'],
    '\x1bOQ': ['\x1b[1', 'Q'],
    '\x1bOR': ['\x1b[1', 'R'],
    '\x1bOS': ['\x1b[1', 'S'],
};

const FN_KEYS = ['\x1bOP', '\x1bOQ', '\x1bOR', '\x1bOS', ...[15, 17, 18, 19, 20, 21, 23, 24].map(code => `\x1b[${code}~`)];
for (const code of [15, 17, 18, 19, 20, 21, 23, 24]) {
    CSI[`\x1b[${code}~`] = [`\x1b[${code}`, '~'];
}

const REPEAT = new Set<string>([SEQ.UP, SEQ.DOWN, SEQ.LEFT, SEQ.RIGHT, SEQ.PGUP, SEQ.PGDN, SEQ.BKSP, SEQ.DEL]);

function modifierParam(ctrl: boolean, alt: boolean, shift: boolean): number {
    return 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0);
}

function applyCtrlChar(codePoint: number): number {
    if (codePoint >= 97 && codePoint <= 122) {
        return codePoint - 96;
    }
    if (codePoint >= 65 && codePoint <= 90) {
        return codePoint - 64;
    }
    if (codePoint === 32 || codePoint === 50) {
        return 0;
    }
    if (codePoint === 91 || codePoint === 51) {
        return 27;
    }
    if (codePoint === 92 || codePoint === 52) {
        return 28;
    }
    if (codePoint === 93 || codePoint === 53) {
        return 29;
    }
    if (codePoint === 94 || codePoint === 54) {
        return 30;
    }
    if (codePoint === 95 || codePoint === 55 || codePoint === 47) {
        return 31;
    }
    if (codePoint === 56) {
        return 127;
    }
    return codePoint;
}

function applyTermux(data: string, ctrl: boolean, alt: boolean, shift: boolean): string {
    if (!ctrl && !alt && !shift) {
        return data;
    }
    const csi = CSI[data];
    if (csi) {
        return `${csi[0]};${modifierParam(ctrl, alt, shift)}${csi[1]}`;
    }
    if (data === SEQ.TAB && shift) {
        return '\x1b[Z';
    }
    if (data === SEQ.TAB || data === SEQ.ESC) {
        return data;
    }
    const units = [...data];
    if (units.length !== 1) {
        return data;
    }
    const raw = units[0].codePointAt(0);
    if (raw === undefined) {
        throw new Error('codePoint missing');
    }
    const body = String.fromCodePoint(ctrl ? applyCtrlChar(raw) : raw);
    return alt ? `${SEQ.ESC}${body}` : body;
}

function stickyOff(): Sticky {
    return {active: false, locked: false};
}

export function TerminalAccessory(props: TerminalAccessoryProps) {
    const {t} = useI18n();
    const pressFiredRef = useRef(false);
    const altRef = useRef<Sticky>(stickyOff());
    const shiftRef = useRef<Sticky>(stickyOff());
    const [shift, setShift] = useState<Sticky>(stickyOff());
    const [fnOpen, setFnOpen] = useState(false);
    const ctrlRef = useRef<Sticky>(stickyOff());
    const pressModRef = useRef<number | null>(null);
    const pressRawRef = useRef<string | null>(null);
    const pressTimerRef = useRef<number | null>(null);
    const [alt, setAlt] = useState<Sticky>(stickyOff());
    const pressIntervalRef = useRef<number | null>(null);
    const pressActRef = useRef<(() => void) | null>(null);
    const [ctrl, setCtrl] = useState<Sticky>(stickyOff());
    const {onHide, onInput, onOpenSearch, onOpenSuggest, ref} = props;

    const writeSticky = (which: number, next: Sticky): void => {
        if (which === MOD_SHIFT) {
            shiftRef.current = next;
            setShift(next);
            return;
        }
        if (which === MOD_CTRL) {
            ctrlRef.current = next;
            setCtrl(next);
            return;
        }
        altRef.current = next;
        setAlt(next);
    };

    const toggleMod = (which: number, lock: boolean): void => {
        const cur = which === MOD_CTRL ? ctrlRef.current : which === MOD_SHIFT ? shiftRef.current : altRef.current;
        writeSticky(which, cur.active ? stickyOff() : {active: true, locked: lock});
    };

    const consume = (data: string): string => {
        try {
            if (data !== SEQ.TAB && data !== SEQ.ESC && !CSI[data] && ([...data].length !== 1 || data.codePointAt(0)! > 127)) {
                return data;
            }
            const ctrlState = ctrlRef.current;
            const altState = altRef.current;
            const next = applyTermux(data, ctrlState.active, altState.active, shiftRef.current.active);
            if (shiftRef.current.active && !shiftRef.current.locked) {
                writeSticky(MOD_SHIFT, stickyOff());
            }
            if (ctrlState.active && !ctrlState.locked) {
                writeSticky(MOD_CTRL, stickyOff());
            }
            if (altState.active && !altState.locked) {
                writeSticky(MOD_ALT, stickyOff());
            }
            return next;
        } catch (error) {
            console.warn(error);
            return data;
        }
    };

    const clearPress = (): void => {
        if (pressTimerRef.current !== null) {
            window.clearTimeout(pressTimerRef.current);
            pressTimerRef.current = null;
        }
        if (pressIntervalRef.current !== null) {
            window.clearInterval(pressIntervalRef.current);
            pressIntervalRef.current = null;
        }
    };

    const armPointer = (event: ReactPointerEvent<HTMLElement>): void => {
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
    };

    const resetPress = (): void => {
        pressFiredRef.current = false;
        pressRawRef.current = null;
        pressModRef.current = null;
        pressActRef.current = null;
    };

    const downSeq = (event: ReactPointerEvent<HTMLElement>, raw: string, repeat: boolean): void => {
        armPointer(event);
        clearPress();
        resetPress();
        pressRawRef.current = raw;
        if (!repeat) {
            return;
        }
        pressTimerRef.current = window.setTimeout(() => {
            pressFiredRef.current = true;
            const repeated = consume(raw);
            onInput(repeated);
            pressIntervalRef.current = window.setInterval(() => {
                onInput(repeated);
            }, REPEAT_MS);
        }, LONG_PRESS_MS);
    };

    const downMod = (event: ReactPointerEvent<HTMLElement>, which: number): void => {
        armPointer(event);
        clearPress();
        resetPress();
        pressModRef.current = which;
        pressTimerRef.current = window.setTimeout(() => {
            pressFiredRef.current = true;
            toggleMod(which, true);
        }, LONG_PRESS_MS);
    };

    const downAct = (event: ReactPointerEvent<HTMLElement>, run: () => void): void => {
        armPointer(event);
        clearPress();
        resetPress();
        pressActRef.current = run;
    };

    const upPress = (event: ReactPointerEvent<HTMLElement>): void => {
        event.preventDefault();
        clearPress();
        if (pressFiredRef.current) {
            return;
        }
        pressFiredRef.current = true;
        const raw = pressRawRef.current;
        if (raw !== null) {
            onInput(raw);
            return;
        }
        const which = pressModRef.current;
        if (which !== null) {
            toggleMod(which, false);
            return;
        }
        const run = pressActRef.current;
        if (run !== null) {
            run();
        }
    };

    const cancelPress = (event: ReactPointerEvent<HTMLElement>): void => {
        event.preventDefault();
        clearPress();
        pressFiredRef.current = true;
    };

    const seqBind = (raw: string) => ({
        onClick: (event: { detail: number }) => {
            if (event.detail === 0) {
                onInput(raw);
            }
        },
        onPointerDown: (event: ReactPointerEvent<HTMLElement>) => downSeq(event, raw, REPEAT.has(raw)),
        onPointerUp: upPress,
        onPointerCancel: cancelPress,
        onLostPointerCapture: cancelPress,
    });

    const modBind = (which: number) => ({
        onClick: (event: { detail: number }) => {
            if (event.detail === 0) {
                toggleMod(which, false);
            }
        },
        onPointerDown: (event: ReactPointerEvent<HTMLElement>) => downMod(event, which),
        onPointerUp: upPress,
        onPointerCancel: cancelPress,
        onLostPointerCapture: cancelPress,
    });

    const actBind = (run: () => void) => ({
        onClick: (event: { detail: number }) => {
            if (event.detail === 0) {
                run();
            }
        },
        onPointerDown: (event: ReactPointerEvent<HTMLElement>) => downAct(event, run),
        onPointerUp: upPress,
        onPointerCancel: cancelPress,
        onLostPointerCapture: cancelPress,
    });

    const keyClass = (active: boolean): string => (active ? 'tm-terminal-accessory-key is-active' : 'tm-terminal-accessory-key');

    useImperativeHandle(ref, () => ({consume}));
    useEffect(() => {
        const blur = (): void => {
            clearPress();
            pressFiredRef.current = true;
        };
        window.addEventListener('blur', blur);
        return () => {
            clearPress();
            window.removeEventListener('blur', blur);
        };
    }, []);

    if (fnOpen) {
        return <Box className="tm-terminal-accessory is-fn" data-tm-gesture-ignore="true" data-tm-ime="true">
            <ButtonBase className={keyClass(true)} {...actBind(() => setFnOpen(false))}>Fn</ButtonBase>
            {FN_KEYS.map((seq, index) => <ButtonBase key={seq} className="tm-terminal-accessory-key" {...seqBind(seq)}>F{index + 1}</ButtonBase>)}
            <ButtonBase className={keyClass(ctrl.active)} {...modBind(MOD_CTRL)}>Ctrl</ButtonBase>
            <ButtonBase className={keyClass(alt.active)} {...modBind(MOD_ALT)}>Alt</ButtonBase>
            <ButtonBase className={keyClass(shift.active)} {...modBind(MOD_SHIFT)}>Shift</ButtonBase>
            <ButtonBase className="tm-terminal-accessory-key is-hide" aria-label={t('protocol.terminal.hideKeyboard')} {...actBind(onHide)}><KeyboardHide fontSize="inherit"/></ButtonBase>
        </Box>;
    }

    return (
        <Box className="tm-terminal-accessory" data-tm-gesture-ignore="true" data-tm-ime="true">
            <ButtonBase className="tm-terminal-accessory-key" {...actBind(() => setFnOpen(true))}>Fn</ButtonBase>
            <ButtonBase className="tm-terminal-accessory-key" {...seqBind(SEQ.BKSP)}>BKSP</ButtonBase>
            <ButtonBase className="tm-terminal-accessory-key" {...seqBind(SEQ.DEL)}>DEL</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.ESC)}>Esc</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind('/')}>/</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind('-')}>-</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.HOME)}>Home</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind(SEQ.UP)}>{'\u2191'}</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.END)}>End</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.PGUP)}>PgUp</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...actBind(onOpenSearch)}>{t('protocol.terminal.searchFind')}</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-hide" aria-label={t('protocol.terminal.hideKeyboard')} {...actBind(onHide)}><KeyboardHide fontSize="inherit"/></ButtonBase>
            <ButtonBase className="tm-terminal-accessory-key is-space" {...seqBind(SEQ.SPACE)}>SPACE</ButtonBase>
            <ButtonBase className={keyClass(shift.active)} {...modBind(MOD_SHIFT)}>Shift</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.TAB)}>Tab</ButtonBase>
            <ButtonBase disableRipple className={keyClass(ctrl.active)} {...modBind(MOD_CTRL)}>Ctrl</ButtonBase>
            <ButtonBase disableRipple className={keyClass(alt.active)} {...modBind(MOD_ALT)}>Alt</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind(SEQ.LEFT)}>{'\u2190'}</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind(SEQ.DOWN)}>{'\u2193'}</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key is-sym" {...seqBind(SEQ.RIGHT)}>{'\u2192'}</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...seqBind(SEQ.PGDN)}>PgDn</ButtonBase>
            <ButtonBase disableRipple className="tm-terminal-accessory-key" {...actBind(onOpenSuggest)}>{t('protocol.terminal.accessorySuggest')}</ButtonBase>
        </Box>
    );
}
