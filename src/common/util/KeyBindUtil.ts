export type KeyBind = {
    key: string;
    code: string;
    ctrl: boolean;
    alt: boolean;
    shift: boolean;
    meta: boolean;
};

export type KeyBindEvent = {
    key: string;
    code: string;
    ctrlKey: boolean;
    altKey: boolean;
    shiftKey: boolean;
    metaKey: boolean;
};

export function eventToKeyBind(event: KeyBindEvent): KeyBind {
    return {
        key: event.key,
        code: event.code,
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
    };
}

export function formatKeyBind(bind: KeyBind): string {
    const names: string[] = [];
    if (bind.ctrl) {
        names.push('Ctrl');
    }
    if (bind.alt) {
        names.push('Alt');
    }
    if (bind.shift) {
        names.push('Shift');
    }
    if (bind.meta) {
        names.push('Meta');
    }
    names.push(formatKeyName(bind.key));
    return names.join('+');
}

export function keyBindEquals(left: KeyBind, right: KeyBind): boolean {
    return left.key === right.key
        && left.code === right.code
        && left.ctrl === right.ctrl
        && left.alt === right.alt
        && left.shift === right.shift
        && left.meta === right.meta;
}

export function serializeKeyBind(bind: KeyBind): string {
    return JSON.stringify(bind);
}

export function deserializeKeyBind(raw: string): KeyBind {
    const parsed = JSON.parse(raw) as KeyBind;
    if (parsed === null) {
        throw new Error(`key bind deserialize invalid: ${raw}`);
    }
    return parsed;
}

export function keyBindToSendText(bind: KeyBind): string {
    if (bind.ctrl || bind.alt || bind.meta) {
        throw new Error(`key bind send text rejects modifiers: ${formatKeyBind(bind)}`);
    }
    if (bind.key === ' ') {
        return ' ';
    }
    if (bind.key === 'Enter') {
        return '\r';
    }
    if (bind.key === 'Tab') {
        return '\t';
    }
    if (bind.key.length === 1) {
        return bind.key;
    }
    throw new Error(`key bind send text unsupported: ${bind.key}`);
}

export function sendTextToKeyBind(text: string): KeyBind {
    if (text === ' ') {
        return {key: ' ', code: 'Space', ctrl: false, alt: false, shift: false, meta: false};
    }
    if (text === '\r') {
        return {key: 'Enter', code: 'Enter', ctrl: false, alt: false, shift: false, meta: false};
    }
    if (text === '\t') {
        return {key: 'Tab', code: 'Tab', ctrl: false, alt: false, shift: false, meta: false};
    }
    if (text.length === 1) {
        return {key: text, code: text, ctrl: false, alt: false, shift: false, meta: false};
    }
    throw new Error(`key bind send text invalid: ${JSON.stringify(text)}`);
}

function formatKeyName(key: string): string {
    if (key === ' ') {
        return 'Space';
    }
    if (key.length === 1 && key >= 'a' && key <= 'z') {
        return key.toUpperCase();
    }
    return key;
}
