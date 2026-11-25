import {useEffect, useState} from 'react';
import type {Editor} from '@tiptap/react';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {insertEditorReference} from '@/page/common/util/reference/editor.ts';
import {useAiMcpServerService, useAiSkillService, useDeviceService} from '@/common/context/useAppHooks.tsx';
import {buildHintGroups, buildSlashHintItems, buildTreeHintItems, filterHintItems, flattenHintRows, hintItemReference, hintServerReference, resolveHintQuery, HINT_ROW, HINT_TRIGGER, type HintItem, type HintQuery, type HintServer} from '@/page/agent/main/input/hint/hintItems.ts';

export interface UseInputHintsParams {
    resolveEditor: () => Editor | null;
    applyReference: (ref: ChatReference) => void;
}

export function useInputHints(params: UseInputHintsParams) {
    const deviceService = useDeviceService();
    const aiSkillService = useAiSkillService();
    const {resolveEditor, applyReference} = params;
    const aiMcpServerService = useAiMcpServerService();
    const [items, setItems] = useState<HintItem[]>([]);
    const [expanded, setExpanded] = useState<string[]>([]);
    const [collapsed, setCollapsed] = useState<string[]>([]);
    const [hint, setHint] = useState<HintQuery | null>(null);
    const [activeIndex, setActiveIndex] = useState<number>(0);
    const trigger = hint === null ? null : hint.trigger;
    const groups = hint === null ? [] : buildHintGroups(filterHintItems(items, hint.query), {expanded, collapsed});
    const rows = flattenHintRows(groups);
    const active = Math.min(activeIndex, Math.max(rows.length - 1, 0));

    function update(editor: Editor): void {
        const next = resolveHintQuery(editor);
        if (next === null || hint === null || next.query !== hint.query || next.trigger !== hint.trigger) {
            setActiveIndex(0);
        }
        if (next === null || hint === null || next.trigger !== hint.trigger) {
            setExpanded([]);
            setCollapsed([]);
        }
        setHint(next);
    }

    function handleToggle(key: string): void {
        setCollapsed(prev => (prev.includes(key) ? prev.filter(item => item !== key) : [...prev, key]));
        setActiveIndex(0);
    }

    function applyHintReference(ref: ChatReference): void {
        const editor = resolveEditor();
        if (editor === null || hint === null) {
            return;
        }
        if (hint.range === null) {
            applyReference(ref);
            return;
        }
        editor.chain().focus(null, {scrollIntoView: false}).deleteRange(hint.range).run();
        insertEditorReference(editor, ref, hint.range.from);
    }

    function handlePick(item: HintItem): void {
        applyHintReference(hintItemReference(item));
    }

    function handlePickServer(server: HintServer): void {
        applyHintReference(hintServerReference(server));
    }

    function handleExpand(key: string): void {
        setExpanded(prev => [...prev, key]);
    }

    function moveActive(delta: number): boolean {
        if (rows.length === 0) {
            return false;
        }
        setActiveIndex((active + delta + rows.length) % rows.length);
        return true;
    }

    function pickActive(): boolean {
        const row = rows[active];
        if (row === undefined) {
            return false;
        }
        if (row.type === HINT_ROW.MORE) {
            handleExpand(row.group.key);
            return true;
        }
        if (row.type === HINT_ROW.SERVER && row.group.server !== null) {
            handlePickServer(row.group.server);
            return true;
        }
        if (row.item !== null) {
            handlePick(row.item);
            return true;
        }
        return false;
    }

    function loadItems(): Promise<HintItem[]> {
        if (trigger === HINT_TRIGGER.SLASH) {
            return Promise.all([aiSkillService.selectAll(), aiMcpServerService.listConnectedTools(), aiMcpServerService.listPrompts()]).then(([skills, tools, prompts]) => buildSlashHintItems(skills, tools, prompts));
        }
        return deviceService.selectDeviceTree().then(buildTreeHintItems);
    }

    useEffect(() => {
        if (trigger === null) {
            return;
        }
        let cancelled = false;
        loadItems().then(list => {
            if (cancelled) {
                return;
            }
            setItems(list);
        }).catch(err => {
            console.error('[input] load hint items error', err);
        });
        return () => {
            cancelled = true;
        };
    }, [trigger, aiSkillService, aiMcpServerService, deviceService]);

    return {trigger, groups, active, update, moveActive, pickActive, handlePick, handlePickServer, handleExpand, handleToggle};
}
