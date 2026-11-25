import {HOME_MENU} from '@/page/common/model/HomeMenu';
import type {TabManagerService} from '@/common/components/tab/service/TabManagerService.ts';

export const GUIDANCE_BEAK_H = 7;
export const GUIDANCE_BEAK_W = 14;
export const GUIDANCE_SPOT_PAD = 5;
export const GUIDANCE_BEAK_TOP = 0;
export const GUIDANCE_BEAK_LEFT = 1;
export const GUIDANCE_BEAK_BOTTOM = 2;
export const GUIDANCE_TIP_WIDTH = 280;
export const GUIDANCE_TAB_DECK = 'tm-tab-deck';

export const GUIDANCE_STEPS = [
    {id: 'home', titleKey: 'guidance.steps.home.title', bodyKey: 'guidance.steps.home.body'},
    {id: 'devices', titleKey: 'guidance.steps.devices.title', bodyKey: 'guidance.steps.devices.body'},
    {id: 'keys', titleKey: 'guidance.steps.keys.title', bodyKey: 'guidance.steps.keys.body'},
    {id: 'forward', titleKey: 'guidance.steps.forward.title', bodyKey: 'guidance.steps.forward.body'},
    {id: 'jobs', titleKey: 'guidance.steps.jobs.title', bodyKey: 'guidance.steps.jobs.body'},
    {id: 'deck', titleKey: 'guidance.steps.deck.title', bodyKey: 'guidance.steps.deck.body'},
    {id: 'add', titleKey: 'guidance.steps.add.title', bodyKey: 'guidance.steps.add.body'},
    {id: 'avatar', titleKey: 'guidance.steps.avatar.title', bodyKey: 'guidance.steps.avatar.body'},
] as const;

export const GUIDANCE_DECK_INPUT_STEPS = [
    {id: 'import', titleKey: 'guidance.deckInput.import.title', bodyKey: 'guidance.deckInput.import.body'},
    {id: 'mention', titleKey: 'guidance.deckInput.mention.title', bodyKey: 'guidance.deckInput.mention.body'},
] as const;

export interface GuidanceTarget {
    left: number;
    top: number;
    width: number;
    height: number;
    radius: string;
}

export interface GuidanceTabs {
    showHomeTab(): void;
    showDeckTab(): void;
}

let tabs: GuidanceTabs | null = null;
let agentNewChatFn: (() => void) | null = null;
let showHomeMenuFn: ((menu: number) => void) | null = null;

export function registerGuidanceTabs(next: GuidanceTabs): () => void {
    tabs = next;
    return () => {
        if (tabs === next) {
            tabs = null;
        }
    };
}

export function registerGuidanceHomeMenu(fn: (menu: number) => void): () => void {
    showHomeMenuFn = fn;
    return () => {
        if (showHomeMenuFn === fn) {
            showHomeMenuFn = null;
        }
    };
}

export function guidanceShowHomeTab(): void {
    if (tabs === null) {
        throw new Error('guidance tabs missing');
    }
    tabs.showHomeTab();
}

export function guidanceShowDeckTab(): void {
    if (tabs === null) {
        throw new Error('guidance tabs missing');
    }
    tabs.showDeckTab();
}

export function guidanceShowHomeMenu(menu: number): void {
    if (showHomeMenuFn === null) {
        throw new Error('guidance home menu missing');
    }
    showHomeMenuFn(menu);
}

export function registerGuidanceAgentNewChat(fn: () => void): () => void {
    agentNewChatFn = fn;
    return () => {
        if (agentNewChatFn === fn) {
            agentNewChatFn = null;
        }
    };
}

export function guidanceAgentNewChat(): void {
    if (agentNewChatFn === null) {
        throw new Error('guidance agent new chat missing');
    }
    agentNewChatFn();
}

export function guidanceHasHomeMenu(): boolean {
    return showHomeMenuFn !== null;
}

export function guidanceHasTabs(): boolean {
    return tabs !== null;
}

function queryTab(tabId: string): HTMLElement | null {
    const match = document.querySelector(`[data-tab-id="${tabId}"]`);
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

function queryNav(menu: number): HTMLElement | null {
    const match = document.querySelector(`[data-tm-nav-menu="${menu}"]`);
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

function queryAddHost(): HTMLElement | null {
    const match = document.querySelector('[data-tm-add-host="true"]');
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

function queryAvatar(): HTMLElement | null {
    const match = document.querySelector('[data-tm-bar-avatar="true"]');
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

export function guidanceQueryAnchor(id: string): HTMLElement | null {
    if (id === 'home') {
        return queryTab('tm-tab-home');
    }
    if (id === 'deck') {
        return queryTab(GUIDANCE_TAB_DECK);
    }
    if (id === 'devices') {
        return queryNav(HOME_MENU.DEVICES);
    }
    if (id === 'keys') {
        return queryNav(HOME_MENU.KEYS);
    }
    if (id === 'forward') {
        return queryNav(HOME_MENU.FORWARD);
    }
    if (id === 'jobs') {
        return queryNav(HOME_MENU.JOBS);
    }
    if (id === 'add') {
        return queryAddHost();
    }
    if (id === 'avatar') {
        return queryAvatar();
    }
    throw new Error(`guidance step unknown: ${id}`);
}

export function guidanceNeedsHomeMenu(id: string): boolean {
    return id !== 'home' && id !== 'deck' && id !== 'avatar';
}

export function guidanceApplyTab(id: string): void {
    if (id === 'deck') {
        guidanceShowDeckTab();
        return;
    }
    guidanceShowHomeTab();
}

export function guidanceApplyMenu(id: string): void {
    if (id === 'home' || id === 'deck' || id === 'avatar') {
        return;
    }
    if (id === 'devices' || id === 'add') {
        guidanceShowHomeMenu(HOME_MENU.DEVICES);
        return;
    }
    if (id === 'keys') {
        guidanceShowHomeMenu(HOME_MENU.KEYS);
        return;
    }
    if (id === 'forward') {
        guidanceShowHomeMenu(HOME_MENU.FORWARD);
        return;
    }
    if (id === 'jobs') {
        guidanceShowHomeMenu(HOME_MENU.JOBS);
        return;
    }
    throw new Error(`guidance surface unknown: ${id}`);
}

export function guidanceIsTabStep(id: string): boolean {
    return id === 'home' || id === 'deck' || id === 'add' || id === 'avatar';
}

export function guidanceHasClosableTab(tabManagerService: TabManagerService): boolean {
    for (const tab of tabManagerService.getTabs()) {
        if (tab.id === undefined) {
            throw new Error('tab id missing');
        }
        const bar = tabManagerService.getTabBar(tab.id);
        if (bar === null) {
            throw new Error(`tab bar missing: ${tab.id}`);
        }
        if (bar.canClose) {
            return true;
        }
    }
    return false;
}

export function guidanceQueryDeckInputEditor(): HTMLElement | null {
    const match = document.querySelector('[data-tm-deck-input-editor="true"]');
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

export function guidanceQueryDeckInputPanel(): HTMLElement | null {
    const match = document.querySelector('[data-tm-deck-input-panel="true"]');
    if (!(match instanceof HTMLElement)) {
        return null;
    }
    return match;
}

export function guidanceReadTarget(el: HTMLElement): GuidanceTarget | null {
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
        return null;
    }
    return {
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        radius: getComputedStyle(el).borderRadius,
    };
}
