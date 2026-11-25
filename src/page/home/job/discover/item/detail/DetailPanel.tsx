import {ProcessPanel} from './ProcessPanel.tsx';
import {ArrowForward} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {CandidateList} from './CandidateList.tsx';
import {generateId} from '@/common/util/IdUtil.ts';
import type {DiscoverRow} from '../DiscoverList.tsx';
import {formatTime} from '@/page/common/util/jobUtil.ts';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {useEffect, useMemo, useRef, useState} from 'react';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {Box, Button, ButtonBase, Typography} from '@mui/material';
import {AgentMarkdown} from '@/page/agent/common/AgentMarkdown.tsx';
import {AI_TOOL_ID, resolveAiToolId} from '@/common/const/AiToolConst.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {isDiscoverRunning, type JobDiscoverItem} from '@/common/model/JobDiscover.ts';
import {useAgentService, useJobDiscoverService} from '@/common/context/useAppHooks.tsx';
import {JOB_DISCOVER_DRAWER_TAB, type JobDiscoverDrawerTab} from '@/page/common/model/JobModel';

interface DetailPanelProps {
    open: boolean;
    onClose: () => void;
    onAccepted?: () => void;
    row: DiscoverRow | null;
}

function isRunEnd(type: number): boolean {
    return type === RESP_EVENT_AGENT.ON_DONE || type === RESP_EVENT_AGENT.ON_ERROR || type === RESP_EVENT_AGENT.ON_CANCELLED;
}

function isDraftDone(item: AiChatItem): boolean {
    if (item.type !== RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
        return false;
    }
    return resolveAiToolId(String(item.context?.toolId ?? '')) === AI_TOOL_ID.JOB_INDICATOR_DRAFT;
}

export function DetailPanel(props: DetailPanelProps) {
    const {t, tf} = useI18n();
    const loadGenRef = useRef(0);
    const [busy, setBusy] = useState(false);
    const agentService = useAgentService();
    const discover = props.row?.discover ?? null;
    const discoverId = discover?.id ?? null;
    const chatId = discover?.chat_id ?? null;
    const {open, row, onClose, onAccepted} = props;
    const jobDiscoverService = useJobDiscoverService();
    const [items, setItems] = useState<JobDiscoverItem[]>([]);
    const sessionId = useMemo(() => generateId(), [discoverId]);
    const [selectedIds, setSelectedIds] = useState<string[]>([]);
    const running = discover != null && isDiscoverRunning(discover.status);
    const [tab, setTab] = useState<JobDiscoverDrawerTab>(() => jobDiscoverService.getDrawerTab());

    const resolveDiscoverStatusLabel = (status: number): string => {
        return t(`home.job.discover.discoverStatus.${status}`);
    };

    const handleTabChange = (next: JobDiscoverDrawerTab): void => {
        setTab(next);
        jobDiscoverService.setDrawerTab(next);
    };

    const toggleOne = (id: string): void => {
        setSelectedIds(prev => (prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id]));
    };

    const reloadItems = (): void => {
        if (isNil(discoverId)) {
            setItems([]);
            return;
        }
        const gen = loadGenRef.current;
        void jobDiscoverService.selectItem(discoverId).then(list => {
            if (gen !== loadGenRef.current) {
                return;
            }
            setItems(list);
        }).catch(() => {
            if (gen !== loadGenRef.current) {
                return;
            }
            setItems([]);
        });
    };

    const handleCancel = (): void => {
        if (isNil(chatId)) {
            return;
        }
        jobDiscoverService.handleCancel(chatId).catch(err => {
            console.error('[discover] cancel error', err);
        });
    };

    const handleAccept = (): void => {
        if (busy || selectedIds.length === 0) {
            return;
        }
        setBusy(true);
        void jobDiscoverService.accept(selectedIds).then(() => {
            setSelectedIds([]);
            reloadItems();
            onAccepted?.();
        }).finally(() => {
            setBusy(false);
        });
    };

    useEffect(() => {
        setSelectedIds([]);
        loadGenRef.current += 1;
        const gen = loadGenRef.current;
        if (isNil(discoverId)) {
            setItems([]);
            return;
        }
        void jobDiscoverService.selectItem(discoverId).then(list => {
            if (gen !== loadGenRef.current) {
                return;
            }
            setItems(list);
        }).catch(() => {
            if (gen !== loadGenRef.current) {
                return;
            }
            setItems([]);
        });
        return () => {
            loadGenRef.current += 1;
        };
    }, [jobDiscoverService, discoverId, row?.discover.status]);

    useEffect(() => {
        if (isNil(chatId)) {
            return;
        }
        const off = agentService.addEventListener(chatId, arg => {
            const item = arg.context.payload as AiChatItem | null;
            if (isNil(item) || item.chat_id !== chatId) {
                return;
            }
            if (isDraftDone(item) || isRunEnd(item.type)) {
                reloadItems();
            }
        });
        return () => {
            off();
        };
    }, [agentService, chatId, jobDiscoverService, discoverId]);

    if (!open || isNil(row) || isNil(chatId) || isNil(discover)) {
        return null;
    }

    return (
        <Box className="tm-job-drawer tm-discover-drawer">
            <Box className="tm-job-drawer-head">
                <Box className="tm-job-drawer-head-main">
                    <Typography className="tm-job-drawer-title">{row.deviceName}</Typography>
                    <Typography className="tm-job-drawer-device">{row.host} · {resolveDiscoverStatusLabel(discover.status)} · {t('home.common.startedAt')} {formatTime(discover.timestamp)}</Typography>
                </Box>
                <ButtonBase disableRipple className="tm-job-icon-btn" onClick={onClose} aria-label={t('home.common.collapse')}>
                    <ArrowForward fontSize="small"/>
                </ButtonBase>
            </Box>
            <Box className="tm-job-drawer-tabs">
                <Box className="tm-job-tabs">
                    <ButtonBase disableRipple className={tab === JOB_DISCOVER_DRAWER_TAB.RESULT ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_DISCOVER_DRAWER_TAB.RESULT)}>
                        <Typography component="span">{t('home.job.discover.tabResult')}</Typography>
                    </ButtonBase>
                    <ButtonBase disableRipple className={tab === JOB_DISCOVER_DRAWER_TAB.ECHO ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_DISCOVER_DRAWER_TAB.ECHO)}>
                        <Typography component="span">{t('home.job.discover.tabLog')}</Typography>
                    </ButtonBase>
                </Box>
            </Box>
            <Box className="tm-job-drawer-body">
                {tab === JOB_DISCOVER_DRAWER_TAB.RESULT ? (
                    <Box className="tm-job-drawer-pane" key={discover.id}>
                        <Box className="tm-job-drawer-scroll tm-discover-result-scroll">
                            {!isEmpty(discover.error) ? (
                                <Typography className="tm-job-muted is-danger">{discover.error}</Typography>
                            ) : null}
                            {!isEmpty(discover.summary) ? (
                                <Box className="tm-discover-section">
                                    <Typography className="tm-discover-section-title">{t('home.job.discover.summaryTitle')}</Typography>
                                    <AgentMarkdown content={discover.summary ?? ''} className="tm-agent-md tm-job-alarm-ai-summary"/>
                                </Box>
                            ) : (
                                <Typography className="tm-job-muted">{running ? t('home.job.discover.summaryRunning') : t('home.job.discover.summaryEmpty')}</Typography>
                            )}
                            <CandidateList running={running} items={items} selectedIds={selectedIds} onToggle={toggleOne}/>
                        </Box>
                        <Box className="tm-job-drawer-actions">
                            <Button disableRipple variant="contained" className="tm-btn tm-btn-primary" onClick={handleAccept} disabled={busy || selectedIds.length === 0}>
                                {tf('home.common.acceptCount', {n: selectedIds.length})}
                            </Button>
                        </Box>
                    </Box>
                ) : (
                    <ProcessPanel chatId={chatId} running={running} sessionId={sessionId} onCancel={handleCancel}/>
                )}
            </Box>
        </Box>
    );
}
