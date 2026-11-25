export const RDP_KEY_KIND_KEY = 0;
export const RDP_KEY_KIND_MOD = 1;
export const RDP_KEY_KIND_MACRO = 2;
export const RDP_KEY_KIND_FN = 3;

export const RDP_MODIFIER_CTRL = 0;
export const RDP_MODIFIER_ALT = 1;
export const RDP_MODIFIER_SHIFT = 2;
export const RDP_MODIFIER_META = 3;

export const RDP_MACRO_CAD = 0;
export const RDP_MACRO_COPY = 1;
export const RDP_MACRO_PASTE = 2;
export const RDP_MACRO_CUT = 3;
export const RDP_MACRO_SELECT_ALL = 4;
export const RDP_MACRO_UNDO = 5;
export const RDP_MACRO_SHOW_DESKTOP = 6;
export const RDP_MACRO_TASK_VIEW = 7;
export const RDP_MACRO_RUN = 8;
export const RDP_MACRO_SEARCH = 9;
export const RDP_MACRO_EXPLORER = 10;
export const RDP_MACRO_SWITCH_WINDOW = 11;
export const RDP_MACRO_CLOSE_WINDOW = 12;
export const RDP_MACRO_TASK_MANAGER = 13;
export const RDP_MACRO_LOCK = 14;

export interface RdpKey {
  kind?: number;
  code?: string;
  label: string;
  sym?: boolean;
  mod?: number;
  macro?: number;
  labelKey?: string;
}

export const RDP_MODIFIERS: number[] = [RDP_MODIFIER_CTRL, RDP_MODIFIER_ALT, RDP_MODIFIER_SHIFT];

export const MODIFIER_CODE: Record<number, string> = {
  [RDP_MODIFIER_CTRL]: 'ControlLeft',
  [RDP_MODIFIER_ALT]: 'AltLeft',
  [RDP_MODIFIER_SHIFT]: 'ShiftLeft',
  [RDP_MODIFIER_META]: 'MetaLeft',
};

export const MACRO_KEYS: Record<number, { mods: number[]; code: string }> = {
  [RDP_MACRO_CAD]: { mods: [RDP_MODIFIER_CTRL, RDP_MODIFIER_ALT], code: 'Delete' },
  [RDP_MACRO_COPY]: { mods: [RDP_MODIFIER_CTRL], code: 'KeyC' },
  [RDP_MACRO_PASTE]: { mods: [RDP_MODIFIER_CTRL], code: 'KeyV' },
  [RDP_MACRO_CUT]: { mods: [RDP_MODIFIER_CTRL], code: 'KeyX' },
  [RDP_MACRO_SELECT_ALL]: { mods: [RDP_MODIFIER_CTRL], code: 'KeyA' },
  [RDP_MACRO_UNDO]: { mods: [RDP_MODIFIER_CTRL], code: 'KeyZ' },
  [RDP_MACRO_SHOW_DESKTOP]: { mods: [RDP_MODIFIER_META], code: 'KeyD' },
  [RDP_MACRO_TASK_VIEW]: { mods: [RDP_MODIFIER_META], code: 'Tab' },
  [RDP_MACRO_RUN]: { mods: [RDP_MODIFIER_META], code: 'KeyR' },
  [RDP_MACRO_SEARCH]: { mods: [RDP_MODIFIER_META], code: 'KeyS' },
  [RDP_MACRO_EXPLORER]: { mods: [RDP_MODIFIER_META], code: 'KeyE' },
  [RDP_MACRO_SWITCH_WINDOW]: { mods: [RDP_MODIFIER_ALT], code: 'Tab' },
  [RDP_MACRO_CLOSE_WINDOW]: { mods: [RDP_MODIFIER_ALT], code: 'F4' },
  [RDP_MACRO_TASK_MANAGER]: { mods: [RDP_MODIFIER_CTRL, RDP_MODIFIER_SHIFT], code: 'Escape' },
  [RDP_MACRO_LOCK]: { mods: [RDP_MODIFIER_META], code: 'KeyL' },
};

const fnToggle: RdpKey = { kind: RDP_KEY_KIND_FN, label: 'Fn' };

export const RDP_BAR_KEYS: RdpKey[] = [
  { kind: RDP_KEY_KIND_MOD, mod: RDP_MODIFIER_CTRL, label: 'Ctrl' },
  { kind: RDP_KEY_KIND_MOD, mod: RDP_MODIFIER_ALT, label: 'Alt' },
  { kind: RDP_KEY_KIND_MOD, mod: RDP_MODIFIER_SHIFT, label: 'Shift' },
  { code: 'Escape', label: 'Esc' },
  { code: 'Tab', label: 'Tab' },
  { code: 'Delete', label: 'Del' },
  { code: 'Enter', label: '\u23ce', sym: true },
  { code: 'MetaLeft', label: '\u229e', sym: true },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_COPY, label: '', labelKey: 'common.copy' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_PASTE, label: '', labelKey: 'common.paste' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_CUT, label: '', labelKey: 'protocol.rdp.cut' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_SELECT_ALL, label: '', labelKey: 'protocol.rdp.selectAll' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_UNDO, label: '', labelKey: 'protocol.rdp.undo' },
  { code: 'ArrowLeft', label: '\u2190', sym: true },
  { code: 'ArrowUp', label: '\u2191', sym: true },
  { code: 'ArrowDown', label: '\u2193', sym: true },
  { code: 'ArrowRight', label: '\u2192', sym: true },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_SHOW_DESKTOP, label: '', labelKey: 'protocol.rdp.showDesktop' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_TASK_VIEW, label: '', labelKey: 'protocol.rdp.taskView' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_RUN, label: '', labelKey: 'protocol.rdp.run' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_SEARCH, label: '', labelKey: 'common.search' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_EXPLORER, label: '', labelKey: 'protocol.rdp.explorer' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_SWITCH_WINDOW, label: '', labelKey: 'protocol.rdp.switchWindow' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_CLOSE_WINDOW, label: '', labelKey: 'protocol.rdp.closeWindow' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_TASK_MANAGER, label: '', labelKey: 'protocol.rdp.taskManager' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_LOCK, label: '', labelKey: 'protocol.rdp.lockScreen' },
  { kind: RDP_KEY_KIND_MACRO, macro: RDP_MACRO_CAD, label: 'Ctrl+Alt+Del' },
  fnToggle,
];

export const RDP_FN_KEYS: RdpKey[] = [
  fnToggle,
  { code: 'F1', label: 'F1' },
  { code: 'F2', label: 'F2' },
  { code: 'F3', label: 'F3' },
  { code: 'F4', label: 'F4' },
  { code: 'F5', label: 'F5' },
  { code: 'F6', label: 'F6' },
  { code: 'F7', label: 'F7' },
  { code: 'F8', label: 'F8' },
  { code: 'F9', label: 'F9' },
  { code: 'F10', label: 'F10' },
  { code: 'F11', label: 'F11' },
  { code: 'F12', label: 'F12' },
];
