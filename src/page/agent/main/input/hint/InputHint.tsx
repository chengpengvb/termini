import {Box, ButtonBase} from '@mui/material';
import {ChevronRight} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {useEffect, useRef, type ReactNode} from 'react';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import type {HintGroup, HintItem, HintServer} from '@/page/agent/main/input/hint/hintItems.ts';

interface InputHintProps {
    active: number;
    groups: HintGroup[];
    trigger: string | null;
    onPick: (item: HintItem) => void;
    onExpand: (key: string) => void;
    onToggle: (key: string) => void;
    onPickServer: (server: HintServer) => void;
}

const GROUP_LABEL_KEY: Record<number, string> = {
    [CHAT_TARGET_TYPE.SKILL]: 'agent.mount.skill',
    [CHAT_TARGET_TYPE.DEVICE]: 'agent.mount.device',
    [CHAT_TARGET_TYPE.GROUP]: 'agent.mount.group',
};

export function InputHint(props: InputHintProps) {
    const {t, tf} = useI18n();
    const rootRef = useRef<HTMLDivElement | null>(null);
    const {active, groups, trigger, onPick, onExpand, onToggle, onPickServer} = props;

    function rowClass(index: number, base: string): string {
        return index === active ? `${base} is-active` : base;
    }

    function renderRow(item: HintItem, index: number) {
        return (
            <ButtonBase key={`${item.kind}:${item.id}`} disableRipple className={rowClass(index, 'tm-agent-hint-row')} data-tm-hint-active={index === active ? 'true' : undefined} onClick={() => onPick(item)}>
                <Box component="span" className="tm-agent-hint-name">{trigger}{item.name}</Box>
                {isNotEmpty(item.description) ? <Box component="span" className="tm-agent-hint-desc">{item.description}</Box> : null}
            </ButtonBase>
        );
    }

    function renderMore(group: HintGroup, index: number) {
        return (
            <ButtonBase key={`more:${group.key}`} disableRipple className={rowClass(index, 'tm-agent-hint-row is-more')} data-tm-hint-active={index === active ? 'true' : undefined} onClick={() => onExpand(group.key)}>
                <Box component="span" className="tm-agent-hint-desc">{tf('agent.slash.more', {count: group.moreCount})}</Box>
            </ButtonBase>
        );
    }

    function renderRows(group: HintGroup, start: number) {
        return (
            <>
                {group.items.map((item, offset) => renderRow(item, start + offset))}
                {group.moreCount > 0 ? renderMore(group, start + group.items.length) : null}
            </>
        );
    }

    function renderKindGroup(group: HintGroup, start: number) {
        return (
            <Box key={group.key} className="tm-agent-hint-group">
                <Box className="tm-agent-hint-title">{t(GROUP_LABEL_KEY[group.kind])}</Box>
                {renderRows(group, start)}
            </Box>
        );
    }

    function renderServerGroup(group: HintGroup, server: HintServer, start: number) {
        return (
            <Box key={group.key} className="tm-agent-hint-group is-server">
                <Box className={rowClass(start, 'tm-agent-hint-parent-row')}>
                    <ButtonBase disableRipple className="tm-agent-hint-parent-main" title={tf('agent.slash.server', {name: server.name})} onClick={() => onPickServer(server)}>
                        <Box component="span" className="tm-agent-hint-parent-head">
                            <Box component="span" className="tm-agent-hint-name">{trigger}{server.name}</Box>
                            <Box component="span" className="tm-agent-hint-parent-count">{group.total}</Box>
                        </Box>
                        {isNotEmpty(group.description) ? <Box component="span" className="tm-agent-hint-desc">{group.description}</Box> : null}
                    </ButtonBase>
                    <ButtonBase disableRipple className="tm-agent-hint-parent-toggle" aria-label={group.collapsed ? t('home.expand') : t('home.collapse')} onClick={() => onToggle(group.key)}>
                        <ChevronRight className={group.collapsed ? 'tm-nav-item-chevron' : 'tm-nav-item-chevron is-expanded'}/>
                    </ButtonBase>
                </Box>
                {group.collapsed ? null : <Box className="tm-agent-hint-children">{renderRows(group, start + 1)}</Box>}
            </Box>
        );
    }

    function renderServersMore(group: HintGroup, index: number) {
        return (
            <ButtonBase key={group.key} disableRipple className={rowClass(index, 'tm-agent-hint-row is-more')} data-tm-hint-active={index === active ? 'true' : undefined} onClick={() => onExpand(group.key)}>
                <Box component="span" className="tm-agent-hint-desc">{tf('agent.slash.more', {count: group.moreCount})}</Box>
            </ButtonBase>
        );
    }

    function renderGroups() {
        const nodes: ReactNode[] = [];
        let index = 0;
        let serverTitleDone = false;
        for (const group of groups) {
            const start = index;
            index += group.items.length + (group.moreCount > 0 ? 1 : 0) + (group.server !== null ? 1 : 0);
            if (group.server === null && !group.serversMore) {
                nodes.push(renderKindGroup(group, start));
                continue;
            }
            if (!serverTitleDone) {
                serverTitleDone = true;
                nodes.push(<Box key="title:mcp" className="tm-agent-hint-title">MCP</Box>);
            }
            if (group.serversMore) {
                nodes.push(renderServersMore(group, start));
                continue;
            }
            nodes.push(renderServerGroup(group, group.server as HintServer, start));
        }
        return nodes;
    }

    useEffect(() => {
        rootRef.current?.querySelector('[data-tm-hint-active="true"]')?.scrollIntoView({block: 'nearest'});
    }, [active, groups]);

    if (groups.length === 0) {
        return null;
    }
    return (
        <Box ref={rootRef} className="tm-agent-hint">
            {renderGroups()}
        </Box>
    );
}
