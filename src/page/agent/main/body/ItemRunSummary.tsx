import {createElement, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Collapse} from '@mui/material';
import {AI_TOOL_ID} from '@/common/const/AiToolConst.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import {useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {FileChangePage} from '@/page/agent/filechange/FileChangePage.tsx';
import {Difference, KeyboardArrowDown, KeyboardArrowRight} from '@mui/icons-material';
import {RUN_SUMMARY_CONFIRM, type RunSummaryLine} from '@/page/common/util/agentRunSummary.ts';

interface ItemRunSummaryProps {
    lines: RunSummaryLine[];
    chatId: string | null;
}

const CONFIRM_KEY: Record<number, string> = {
    [RUN_SUMMARY_CONFIRM.PENDING]: 'agent.tool.runSummaryPending',
    [RUN_SUMMARY_CONFIRM.ALLOW]: 'agent.tool.runSummaryAllow',
    [RUN_SUMMARY_CONFIRM.REJECT]: 'agent.tool.runSummaryReject',
};

const LINE_STATE = {
    DONE: 0,
    OK: 1,
    FAIL: 2,
    PENDING: 3,
} as const;

const STATE_CLASS: Record<number, string> = {
    [LINE_STATE.DONE]: 'tm-agent-run-summary-dot',
    [LINE_STATE.OK]: 'tm-agent-run-summary-dot is-ok',
    [LINE_STATE.FAIL]: 'tm-agent-run-summary-dot is-fail',
    [LINE_STATE.PENDING]: 'tm-agent-run-summary-dot is-pending',
};

function resolveLineState(line: RunSummaryLine): number {
    if (line.confirm === RUN_SUMMARY_CONFIRM.REJECT) {
        return LINE_STATE.FAIL;
    }
    if (line.confirm === RUN_SUMMARY_CONFIRM.PENDING) {
        return LINE_STATE.PENDING;
    }
    if (line.killed) {
        return LINE_STATE.FAIL;
    }
    if (isNil(line.exitCode)) {
        return LINE_STATE.DONE;
    }
    if (line.exitCode === 0) {
        return LINE_STATE.OK;
    }
    return LINE_STATE.FAIL;
}

export function ItemRunSummary(props: ItemRunSummaryProps) {
    const {t, tf} = useI18n();
    const {lines, chatId} = props;
    const [open, setOpen] = useState(false);
    const tabManagerService = useTabManagerService();
    const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(new Set());

    if (lines.length === 0) {
        return null;
    }

    function isEditLine(line: RunSummaryLine): boolean {
        return line.toolId === AI_TOOL_ID.SEARCH_REPLACE || line.toolId === AI_TOOL_ID.SFTP_SEARCH_REPLACE;
    }

    function handleOpenFileChange(line: RunSummaryLine): void {
        if (isNil(chatId)) {
            return;
        }
        tabManagerService.openTab({
            panes: [{
                tabBar: {
                    canClose: true,
                    title: t('agent.fileChange.tabTitle'),
                    icon: createElement(Difference, {sx: {fontSize: 16}}),
                },
                content: createElement(FileChangePage, {chatId, focusPath: line.targetFull}),
            }],
        });
    }

    function resolveMeta(line: RunSummaryLine): string[] {
        const parts: string[] = [];
        if (isNotNil(line.confirm)) {
            const key = CONFIRM_KEY[line.confirm];
            if (key) {
                parts.push(t(key));
            }
        }
        if (line.killed) {
            parts.push('killed');
        } else if (isNotNil(line.exitCode)) {
            parts.push(`exit ${line.exitCode}`);
        }
        return parts;
    }

    function resolveTitle(): string {
        return tf('agent.tool.runSummaryTitle', {n: lines.length});
    }

    function handleToggleOpen(): void {
        setOpen((current: boolean) => !current);
    }

    function canExpandLine(line: RunSummaryLine): boolean {
        return isNotNil(line.targetFull) || isNotNil(line.resultFull);
    }

    function handleToggleLine(callId: string): void {
        setExpandedIds((current: ReadonlySet<string>) => {
            const next = new Set(current);
            if (next.has(callId)) {
                next.delete(callId);
            } else {
                next.add(callId);
            }
            return next;
        });
    }

    function renderLineRow(line: RunSummaryLine) {
        const edit = isEditLine(line);
        const showAdd = isNotNil(line.addCount) && line.addCount > 0;
        const showDel = isNotNil(line.delCount) && line.delCount > 0;
        if (edit) {
            return (
                <>
                    <Box component="span" className="tm-agent-run-summary-main">
                        {line.target ? <Box component="span" className="tm-agent-run-summary-target">{line.target}</Box> : null}
                    </Box>
                    <Box className="tm-agent-run-summary-aside">
                        {showAdd || showDel ? (
                            <Box className="tm-agent-diff-stats">
                                {showAdd ? <Box component="span" className="tm-agent-diff-stat-add">+{line.addCount}</Box> : null}
                                {showDel ? <Box component="span" className="tm-agent-diff-stat-del">-{line.delCount}</Box> : null}
                            </Box>
                        ) : null}
                    </Box>
                </>
            );
        }
        const state = resolveLineState(line);
        const meta = resolveMeta(line);
        return (
            <>
                <Box component="span" className={STATE_CLASS[state]} aria-hidden/>
                <Box component="span" className="tm-agent-run-summary-tool">{line.tool}</Box>
                <Box component="span" className="tm-agent-run-summary-main">
                    {line.target ? <Box component="span" className="tm-agent-run-summary-target">{line.target}</Box> : null}
                    {line.result ? <Box component="span" className={state === LINE_STATE.FAIL ? 'tm-agent-run-summary-result is-fail' : 'tm-agent-run-summary-result'}>{line.result}</Box> : null}
                </Box>
                <Box className="tm-agent-run-summary-aside">
                    {meta.length > 0 ? <Box component="span" className="tm-agent-run-summary-meta">{meta.join(' · ')}</Box> : null}
                </Box>
            </>
        );
    }

    function renderLineDetail(line: RunSummaryLine) {
        return (
            <Box className="tm-agent-run-summary-detail">
                {isNotNil(line.targetFull) ? <Box component="pre" className="tm-agent-run-summary-detail-block">{line.targetFull}</Box> : null}
                {isNotNil(line.resultFull) ? <Box component="pre" className="tm-agent-run-summary-detail-block is-result">{line.resultFull}</Box> : null}
            </Box>
        );
    }

    function renderLine(line: RunSummaryLine, index: number) {
        const rowKey = `${line.callId}:${line.targetFull ?? ''}:${index}`;
        if (isEditLine(line) && isNotNil(chatId)) {
            return (
                <Box key={rowKey} className="tm-agent-run-summary-item" component="li">
                    <ButtonBase disableRipple className="tm-agent-run-summary-row is-clickable" onClick={() => handleOpenFileChange(line)}>
                        {renderLineRow(line)}
                    </ButtonBase>
                </Box>
            );
        }
        if (!canExpandLine(line)) {
            return (
                <Box key={rowKey} className="tm-agent-run-summary-item" component="li">
                    <Box className="tm-agent-run-summary-row">{renderLineRow(line)}</Box>
                </Box>
            );
        }
        const expanded = expandedIds.has(line.callId);
        return (
            <Box key={rowKey} className="tm-agent-run-summary-item" component="li">
                <ButtonBase disableRipple className="tm-agent-run-summary-row is-clickable" onClick={() => handleToggleLine(line.callId)}>
                    {renderLineRow(line)}
                </ButtonBase>
                <Collapse in={expanded} unmountOnExit timeout={0}>
                    {renderLineDetail(line)}
                </Collapse>
            </Box>
        );
    }

    return (
        <Box className="tm-agent-run-summary">
            <Box className="tm-agent-run-summary-head">
                <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
                    <Box component="span" className="tm-agent-tool-chevron">
                        {open ? <KeyboardArrowDown sx={{fontSize: 16}}/> : <KeyboardArrowRight sx={{fontSize: 16}}/>}
                    </Box>
                    <Box component="span" className="tm-agent-run-summary-title">{resolveTitle()}</Box>
                </ButtonBase>
            </Box>
            <Collapse in={open} unmountOnExit timeout={0}>
                <Box className="tm-agent-run-summary-list" component="ul">
                    {lines.map((line, index) => renderLine(line, index))}
                </Box>
            </Collapse>
        </Box>
    );
}
