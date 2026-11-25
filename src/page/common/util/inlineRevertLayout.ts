import {isNotNil} from '@/common/util/StringUtil.ts';

const REVERT_VIEW_MARGIN = 12;

export interface InlineRevertBox {
    top: number;
    left: number;
    width: number;
}

export function resolveFixedContainingBlock(el: HTMLElement): HTMLElement {
    let node: HTMLElement | null = el.parentElement;
    while (node && node !== document.documentElement) {
        const style = getComputedStyle(node);
        if (style.transform !== 'none' || style.perspective !== 'none' || style.filter !== 'none') {
            return node;
        }
        node = node.parentElement;
    }
    return document.documentElement;
}

export function resolveRevertBubble(itemId: string): HTMLElement | null {
    const bubble = document.querySelector(`[data-agent-user-item="${itemId}"] [data-tm-user-bubble="true"]`);
    if (!(bubble instanceof HTMLElement)) {
        return null;
    }
    return bubble;
}

export function resolveInlineRevertBox(wrap: HTMLElement, bubble: HTMLElement, topOverride: number | null): InlineRevertBox | null {
    const content = document.querySelector('[data-tm-message-content="true"]');
    if (!(content instanceof HTMLElement)) {
        return null;
    }
    const containingRect = resolveFixedContainingBlock(wrap).getBoundingClientRect();
    const bubbleRect = bubble.getBoundingClientRect();
    const contentRect = content.getBoundingClientRect();
    const top = isNotNil(topOverride) ? topOverride : Math.round(bubbleRect.top - containingRect.top);
    return {
        top: top,
        left: Math.round(contentRect.left - containingRect.left),
        width: Math.round(contentRect.width),
    };
}

function resolveBottomLimit(body: HTMLElement, followup: HTMLElement | null): number {
    if (followup) {
        return followup.getBoundingClientRect().top - REVERT_VIEW_MARGIN;
    }
    return body.getBoundingClientRect().bottom - REVERT_VIEW_MARGIN;
}

export function ensureInlineRevertInView(wrap: HTMLElement, bubble: HTMLElement, followup: HTMLElement | null): InlineRevertBox | null {
    const body = document.querySelector('[data-tm-message-body="true"]');
    let box = resolveInlineRevertBox(wrap, bubble, null);
    if (!box) {
        return null;
    }
    if (!(body instanceof HTMLElement)) {
        return box;
    }
    const wrapHeight = wrap.getBoundingClientRect().height;
    const bodyRect = body.getBoundingClientRect();
    let containingTop = resolveFixedContainingBlock(wrap).getBoundingClientRect().top;
    let limit = resolveBottomLimit(body, followup);
    let screenBottom = containingTop + box.top + wrapHeight;
    if (screenBottom > limit) {
        body.scrollTop += screenBottom - limit;
        containingTop = resolveFixedContainingBlock(wrap).getBoundingClientRect().top;
        box = resolveInlineRevertBox(wrap, bubble, null);
        if (!box) {
            return null;
        }
        limit = resolveBottomLimit(body, followup);
        screenBottom = containingTop + box.top + wrapHeight;
        if (screenBottom > limit) {
            box = resolveInlineRevertBox(wrap, bubble, Math.max(0, Math.round(limit - wrapHeight - containingTop)));
            if (!box) {
                return null;
            }
        }
    }
    containingTop = resolveFixedContainingBlock(wrap).getBoundingClientRect().top;
    const screenTop = containingTop + box.top;
    if (screenTop < bodyRect.top + 12) {
        body.scrollTop -= bodyRect.top + 12 - screenTop;
        box = resolveInlineRevertBox(wrap, bubble, null);
        if (!box) {
            return null;
        }
    }
    return box;
}
