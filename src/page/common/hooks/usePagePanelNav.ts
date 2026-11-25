import type {AnimationEvent} from 'react';
import {useEffect, useRef, useState} from 'react';
import {PAGE_NAV_DIRECTION, type PageNavDirection} from '@/page/common/model/PageNav.ts';

const PANEL_BASE_CLASS = 'tm-page-panel';

function resolveDirectionClass(kind: 'enter' | 'leave', direction: PageNavDirection): string {
    if (kind === 'enter') {
        if (direction === PAGE_NAV_DIRECTION.FORWARD) {
            return 'is-enter-forward';
        }
        return 'is-enter-back';
    }
    if (direction === PAGE_NAV_DIRECTION.FORWARD) {
        return 'is-leave-forward';
    }
    return 'is-leave-back';
}

function prefersReducedMotion(): boolean {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function usePagePanelNav(activeKey: number, enabled: boolean): {
    resolvePanelProps: (panelKey: number, extraClassName?: string | null) => {
        className: string;
        'data-tm-page-panel': 'true';
        'data-tm-page-panel-active': 'true' | 'false';
    };
    onPanelAnimationEnd: (panelKey: number, event: AnimationEvent<HTMLElement>) => void;
} {
    const prevKeyRef = useRef(activeKey);
    const [animating, setAnimating] = useState(false);
    const [renderedKey, setRenderedKey] = useState(activeKey);
    const [leavingKey, setLeavingKey] = useState<number | null>(null);
    const [direction, setDirection] = useState<PageNavDirection>(PAGE_NAV_DIRECTION.FORWARD);

    useEffect(() => {
        if (activeKey === prevKeyRef.current) {
            return;
        }
        const prev = prevKeyRef.current;
        prevKeyRef.current = activeKey;
        if (!enabled || prefersReducedMotion()) {
            setRenderedKey(activeKey);
            setLeavingKey(null);
            setAnimating(false);
            return;
        }
        setDirection(activeKey > prev ? PAGE_NAV_DIRECTION.FORWARD : PAGE_NAV_DIRECTION.BACK);
        setLeavingKey(prev);
        setRenderedKey(activeKey);
        setAnimating(true);
    }, [activeKey, enabled]);

    const resolveClassName = (panelKey: number, extraClassName?: string | null): string => {
        const parts: string[] = [PANEL_BASE_CLASS];
        if (extraClassName != null) {
            parts.push(extraClassName);
        }
        if (!enabled) {
            if (panelKey === activeKey) {
                parts.push('is-active');
            }
            return parts.join(' ');
        }
        const isActive = panelKey === renderedKey;
        const isLeaving = panelKey === leavingKey;
        if (isActive) {
            parts.push('is-active');
            if (animating) {
                parts.push(resolveDirectionClass('enter', direction));
            }
        } else if (isLeaving && animating) {
            parts.push(resolveDirectionClass('leave', direction));
        }
        return parts.join(' ');
    };

    const resolvePanelProps = (panelKey: number, extraClassName?: string | null) => {
        const active = !enabled ? panelKey === activeKey : panelKey === renderedKey;
        return {
            className: resolveClassName(panelKey, extraClassName),
            'data-tm-page-panel': 'true' as const,
            'data-tm-page-panel-active': (active ? 'true' : 'false') as 'true' | 'false',
        };
    };

    const onPanelAnimationEnd = (panelKey: number, event: AnimationEvent<HTMLElement>): void => {
        if (event.target !== event.currentTarget) {
            return;
        }
        if (panelKey !== renderedKey) {
            return;
        }
        setLeavingKey(null);
        setAnimating(false);
    };

    return {resolvePanelProps, onPanelAnimationEnd};
}
