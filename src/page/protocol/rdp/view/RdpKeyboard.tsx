import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { FitScreen, KeyboardHide } from '@mui/icons-material';
import { type RdpInput } from '@/common/model/req/event/WsRdp.ts';
import { resolveScancodeInput } from '@/page/common/util/rdpUtil.ts';
import { useLayoutEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { MACRO_KEYS, MODIFIER_CODE, RDP_BAR_KEYS, RDP_FN_KEYS, RDP_KEY_KIND_FN, RDP_KEY_KIND_MACRO, RDP_KEY_KIND_MOD, RDP_MODIFIERS, type RdpKey } from './rdpKeyboardLayout.ts';

const STICKY_OFF = 0;
const STICKY_ONCE = 1;
const STICKY_LOCKED = 2;
const LONG_PRESS_MS = 400;

export interface RdpKeyboardProps {
  send: (events: RdpInput[]) => void;
  onHide: () => void;
  onFit: () => void;
}

function initialSticky(): Record<number, number> {
  const state: Record<number, number> = {};
  for (const mod of RDP_MODIFIERS) {
    state[mod] = STICKY_OFF;
  }
  return state;
}

function keepImeFocus(event: ReactPointerEvent<HTMLElement>): void {
  event.preventDefault();
}

export function RdpKeyboard(props: RdpKeyboardProps) {
  const { t } = useI18n();
  const rootRef = useRef<HTMLDivElement>(null);
  const pressFiredRef = useRef<boolean>(false);
  const pressTimerRef = useRef<number | null>(null);
  const [fnOpen, setFnOpen] = useState<boolean>(false);
  const stickyRef = useRef<Record<number, number>>(initialSticky());
  const [sticky, setSticky] = useState<Record<number, number>>(initialSticky());
  const { send, onHide, onFit } = props;

  useLayoutEffect(() => {
    const keyboard = rootRef.current;
    const host = keyboard?.parentElement;
    if (!keyboard || !host) {
      return;
    }
    let measureFrame = 0;
    const measure = (): void => {
      const height = `${keyboard.getBoundingClientRect().height}px`;
      if (host.style.getPropertyValue('--tm-rdp-keyboard-height') !== height) {
        host.style.setProperty('--tm-rdp-keyboard-height', height);
      }
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(measureFrame);
      measureFrame = requestAnimationFrame(measure);
    });
    observer.observe(keyboard);
    measure();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(measureFrame);
      host.style.removeProperty('--tm-rdp-keyboard-height');
    };
  }, []);

  const scan = (code: string, pressed: boolean): RdpInput => {
    const input = resolveScancodeInput(code, pressed);
    if (!input) {
      throw new Error(`rdp keyboard unknown code: ${code}`);
    }
    return input;
  };

  const writeSticky = (next: Record<number, number>): void => {
    stickyRef.current = next;
    setSticky(next);
  };

  const clearOnce = (): void => {
    const current = stickyRef.current;
    const next = { ...current };
    let changed = false;
    for (const mod of RDP_MODIFIERS) {
      if (current[mod] === STICKY_ONCE) {
        next[mod] = STICKY_OFF;
        changed = true;
      }
    }
    if (changed) {
      writeSticky(next);
    }
  };

  const activeMods = (): number[] => RDP_MODIFIERS.filter(mod => stickyRef.current[mod] !== STICKY_OFF);

  const sendCombo = (mods: number[], code: string): void => {
    const events: RdpInput[] = [];
    for (const mod of mods) {
      events.push(scan(MODIFIER_CODE[mod], true));
    }
    events.push(scan(code, true), scan(code, false));
    for (let i = mods.length - 1; i >= 0; i -= 1) {
      events.push(scan(MODIFIER_CODE[mods[i]], false));
    }
    send(events);
    clearOnce();
  };

  const toggleMod = (mod: number, locked: boolean): void => {
    const current = stickyRef.current[mod];
    const next = { ...stickyRef.current };
    if (locked) {
      next[mod] = current === STICKY_LOCKED ? STICKY_OFF : STICKY_LOCKED;
    } else {
      next[mod] = current === STICKY_OFF ? STICKY_ONCE : STICKY_OFF;
    }
    writeSticky(next);
  };

  const clearPress = (): void => {
    if (pressTimerRef.current === null) {
      return;
    }
    window.clearTimeout(pressTimerRef.current);
    pressTimerRef.current = null;
  };

  const handleModDown = (event: ReactPointerEvent<HTMLElement>, mod: number): void => {
    event.preventDefault();
    pressFiredRef.current = false;
    clearPress();
    pressTimerRef.current = window.setTimeout(() => {
      pressFiredRef.current = true;
      toggleMod(mod, true);
    }, LONG_PRESS_MS);
  };

  const handleModUp = (event: ReactPointerEvent<HTMLElement>, mod: number): void => {
    event.preventDefault();
    clearPress();
    if (pressFiredRef.current) {
      return;
    }
    toggleMod(mod, false);
  };

  const handleModCancel = (): void => {
    clearPress();
    pressFiredRef.current = true;
  };

  const handleKeyClick = (key: RdpKey): void => {
    if (key.kind === RDP_KEY_KIND_FN) {
      setFnOpen(open => !open);
      return;
    }
    if (key.kind === RDP_KEY_KIND_MACRO && key.macro !== undefined) {
      const macro = MACRO_KEYS[key.macro];
      sendCombo(macro.mods, macro.code);
      return;
    }
    if (key.code) {
      sendCombo(activeMods(), key.code);
    }
  };

  const keyClass = (key: RdpKey): string => {
    if (key.kind === RDP_KEY_KIND_MOD && key.mod !== undefined) {
      return sticky[key.mod] === STICKY_OFF ? 'tm-rdp-key' : 'tm-rdp-key is-active';
    }
    if (key.kind === RDP_KEY_KIND_FN) {
      return fnOpen ? 'tm-rdp-key is-active' : 'tm-rdp-key';
    }
    return key.sym ? 'tm-rdp-key is-sym' : 'tm-rdp-key';
  };

  const keyLabel = (key: RdpKey): string => {
    if (key.labelKey) {
      return t(key.labelKey);
    }
    return key.label;
  };

  const renderKey = (key: RdpKey, index: number) => {
    if (key.kind === RDP_KEY_KIND_MOD && key.mod !== undefined) {
      const mod = key.mod;
      return (
        <ButtonBase key={index} disableRipple className={keyClass(key)} onPointerDown={event => handleModDown(event, mod)} onPointerUp={event => handleModUp(event, mod)} onPointerCancel={handleModCancel}>
          {key.label}
        </ButtonBase>
      );
    }
    return (
      <ButtonBase key={index} disableRipple className={keyClass(key)} onPointerDown={keepImeFocus} onClick={() => handleKeyClick(key)}>
        {keyLabel(key)}
      </ButtonBase>
    );
  };

  const keys = fnOpen ? RDP_FN_KEYS : RDP_BAR_KEYS;

  return (
    <Box ref={rootRef} className="tm-rdp-keyboard" data-tm-gesture-ignore="true" data-tm-ime="true">
      <Box className="tm-rdp-keyboard-scroll">{keys.map((key, index) => renderKey(key, index))}</Box>
      <ButtonBase disableRipple className="tm-rdp-key tm-rdp-key--fit" aria-label={t('protocol.rdp.fitView')} title={t('protocol.rdp.fitView')} onPointerDown={keepImeFocus} onClick={onFit}>
        <FitScreen fontSize="inherit" />
      </ButtonBase>
      <ButtonBase disableRipple className="tm-rdp-key tm-rdp-key--hide" aria-label={t('protocol.rdp.hideKeyboard')} onPointerDown={keepImeFocus} onClick={onHide}>
        <KeyboardHide fontSize="inherit" />
      </ButtonBase>
    </Box>
  );
}
