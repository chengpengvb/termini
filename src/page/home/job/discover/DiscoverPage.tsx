import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {DetailPanel} from './item/detail/DetailPanel.tsx';
import type {JobDevice} from '@/common/model/JobDevice.ts';
import type {JobDiscover} from '@/common/model/JobDiscover.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {useBackDismiss} from '@/page/common/hooks/useBackDismiss.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {DiscoverList, type DiscoverRow} from './item/DiscoverList.tsx';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import {useOpenTransition} from '@/page/common/hooks/useOpenTransition.ts';
import {useEffect, useLayoutEffect, useMemo, useRef, useState} from 'react';
import {AgentSplitDivider, SPLIT_SIDE} from '@/page/agent/common/AgentSplitDivider.tsx';
import {useAccountService, useJobDeviceService, useJobDiscoverService} from '@/common/context/useAppHooks.tsx';

interface DiscoverPageProps {
    active: boolean;
    focusId?: string | null;
    onAccepted?: () => void;
    onFocusConsumed?: () => void;
}

export function DiscoverPage(props: DiscoverPageProps) {
    const {t, tf} = useI18n();
    const mobileViewport = usePlatformMobile();
    const accountService = useAccountService();
    const rootRef = useRef<HTMLDivElement>(null);
    const drawerRef = useRef<HTMLDivElement>(null);
    const jobDeviceService = useJobDeviceService();
    const jobDiscoverService = useJobDiscoverService();
    const [drawerWidth, setDrawerWidth] = useState(380);
    const [error, setError] = useState<string | null>(null);
    const [confirmDelete, setConfirmDelete] = useState(false);
    const [checkedIds, setCheckedIds] = useState<string[]>([]);
    const {active, focusId, onAccepted, onFocusConsumed} = props;
    const [discovers, setDiscovers] = useState<JobDiscover[]>([]);
    const [jobDevices, setJobDevices] = useState<JobDevice[]>([]);
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const [pendingDeleteIds, setPendingDeleteIds] = useState<string[]>([]);
    const rows = useMemo((): DiscoverRow[] => {
        const deviceById = new Map(jobDevices.map(item => [item.id, item]));
        return discovers.map(discover => {
            const jobDevice = deviceById.get(discover.job_device_id);
            return {
                discover: discover,
                host: jobDevice?.device?.host ?? t('home.common.emDash'),
                deviceName: jobDevice?.device?.name ?? t('home.job.deviceRemoved'),
            };
        });
    }, [discovers, jobDevices, t]);
    const selectedRow = rows.find(row => row.discover.id === selectedId) ?? null;
    const drawerOpen = selectedRow != null;
    const {mounted: drawerMounted, active: drawerActive} = useOpenTransition(drawerOpen, 340);
    const closeDrawer = (): void => {
        setSelectedId(null);
    };
    const drawerRowRef = useRef<DiscoverRow | null>(null);
    if (selectedRow) {
        drawerRowRef.current = selectedRow;
    }
    const panelRow = selectedRow ?? drawerRowRef.current;

    const handleToggle = (id: string): void => {
        setCheckedIds(prev => (prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id]));
    };

    const handleToggleAll = (): void => {
        if (checkedIds.length === rows.length) {
            setCheckedIds([]);
            return;
        }
        setCheckedIds(rows.map(row => row.discover.id));
    };

    const handleSelect = (row: DiscoverRow): void => {
        setSelectedId(prev => (prev === row.discover.id ? null : row.discover.id));
    };

    const handleDelete = (id: string): void => {
        const ids = checkedIds.includes(id) && checkedIds.length > 0 ? checkedIds : [id];
        setPendingDeleteIds(ids);
        setConfirmDelete(true);
    };

    const handleConfirmDelete = (): void => {
        setConfirmDelete(false);
        if (pendingDeleteIds.length === 0) {
            return;
        }
        const ids = pendingDeleteIds;
        void jobDiscoverService.deleteBatch(ids).then(() => {
            setError(null);
            setPendingDeleteIds([]);
            setCheckedIds(prev => prev.filter(id => !ids.includes(id)));
            if (selectedId && ids.includes(selectedId)) {
                setSelectedId(null);
            }
        }).catch(err => {
            setError(resolveErrorText(err));
        });
    };

    useBackDismiss(mobileViewport && drawerOpen, closeDrawer, 'job-discover-detail');

    useEffect(() => {
        void jobDiscoverService.selectAll().then(list => {
            setDiscovers(list);
            setCheckedIds(prev => prev.filter(id => list.some(item => item.id === id)));
            setSelectedId(prev => (prev && list.some(item => item.id === prev) ? prev : null));
        });
        return jobDiscoverService.addJobDiscoverListChangeListener(list => {
            setDiscovers(list);
            setCheckedIds(prev => prev.filter(id => list.some(item => item.id === id)));
            setSelectedId(prev => (prev && list.some(item => item.id === prev) ? prev : null));
        });
    }, [jobDiscoverService]);

    useEffect(() => {
        void jobDeviceService.selectAllDetail().then(setJobDevices);
    }, [jobDeviceService]);

    useEffect(() => {
        if (!active) {
            return;
        }
        void jobDiscoverService.selectAll().then(list => {
            setDiscovers(list);
            setCheckedIds(prev => prev.filter(id => list.some(item => item.id === id)));
            setSelectedId(prev => (prev && list.some(item => item.id === prev) ? prev : null));
        });
        void jobDeviceService.selectAllDetail().then(setJobDevices);
    }, [active, jobDeviceService, jobDiscoverService]);

    useEffect(() => {
        return accountService.addCloudSyncDownloadedListener(() => {
            void jobDeviceService.selectAllDetail().then(setJobDevices);
        });
    }, [accountService, jobDeviceService]);

    useEffect(() => {
        if (isNil(focusId)) {
            return;
        }
        if (discovers.some(item => item.id === focusId)) {
            setSelectedId(focusId);
            onFocusConsumed?.();
        }
    }, [focusId, discovers, onFocusConsumed]);

    useLayoutEffect(() => {
        if (!drawerRef.current) {
            return;
        }
        if (mobileViewport) {
            drawerRef.current.style.width = '100%';
            return;
        }
        drawerRef.current.style.width = `${drawerWidth}px`;
    }, [drawerWidth, mobileViewport]);

    const countLabel = checkedIds.length > 0
        ? `${tf('home.common.totalCount', {n: rows.length})}${tf('home.common.selectedSuffix', {n: checkedIds.length})}`
        : tf('home.common.totalCount', {n: rows.length});

    return (
        <Box ref={rootRef} className="tm-job-monitor-root">
            <Box className="tm-job-page">
                <Box className="tm-job-scroll">
                    <Typography className="tm-job-muted">{countLabel}</Typography>
                    {error ? <Typography className="tm-job-drawer-error">{error}</Typography> : null}
                    <DiscoverList rows={rows} selectedId={selectedId} checkedIds={checkedIds} onToggle={handleToggle} onToggleAll={handleToggleAll} onDelete={handleDelete} onSelect={handleSelect}/>
                </Box>
            </Box>
            <Box ref={drawerRef} className={drawerActive ? 'tm-job-drawer-float is-open' : 'tm-job-drawer-float'}>
                {drawerActive && !mobileViewport ? (
                    <AgentSplitDivider containerRef={rootRef} widthTargetRef={drawerRef} side={SPLIT_SIDE.RIGHT} minSize={320} minMain={440} onWidthChange={setDrawerWidth}/>
                ) : null}
                <DetailPanel open={drawerMounted} row={panelRow} onClose={closeDrawer} onAccepted={onAccepted}/>
            </Box>
            <ConfirmDialog open={confirmDelete} title={t('home.job.discover.deleteTitle')} message={tf('home.job.discover.deleteMessage', {n: pendingDeleteIds.length})} confirmLabel={t('common.delete')} danger onCancel={() => setConfirmDelete(false)} onConfirm={handleConfirmDelete}/>
        </Box>
    );
}
