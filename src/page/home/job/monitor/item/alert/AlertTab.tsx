import {AlertList} from './AlertList.tsx';
import {useEffect, useState} from 'react';
import {AlertDetail} from './AlertDetail.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {Box, Button, Typography} from '@mui/material';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import type {JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';
import {useJobIndicatorAlertService} from '@/common/context/useAppHooks.tsx';
import type {JobDevice, JobDevice2Indicator} from '@/common/model/JobDevice.ts';

export interface AlertTabProps {
    jobDevice: JobDevice;
    link: JobDevice2Indicator;
}

export function AlertTab(props: AlertTabProps) {
    const {t, tf} = useI18n();
    const {jobDevice, link} = props;
    const alertService = useJobIndicatorAlertService();
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [confirmDelete, setConfirmDelete] = useState(false);
    const [checkedIds, setCheckedIds] = useState<string[]>([]);
    const [alerts, setAlerts] = useState<JobIndicatorAlert[]>([]);
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const selected = alerts.find(item => item.id === selectedId) ?? null;
    const indicator = link.job_indicator ?? selected?.job_indicator ?? null;

    const reload = (): void => {
        setLoading(true);
        setError(null);
        void alertService.selectList(jobDevice.id, link.job_indicator_id).then(list => {
            setAlerts(list);
            setCheckedIds(prev => prev.filter(id => list.some(item => item.id === id)));
            setSelectedId(prev => (prev && list.some(item => item.id === prev) ? prev : null));
            setLoading(false);
        }).catch(err => {
            setAlerts([]);
            setLoading(false);
            setError(resolveErrorText(err));
        });
    };

    const handleToggle = (id: string): void => {
        setCheckedIds(prev => (prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id]));
    };

    const handleToggleAll = (): void => {
        if (checkedIds.length === alerts.length) {
            setCheckedIds([]);
            return;
        }
        setCheckedIds(alerts.map(item => item.id));
    };

    const handleConfirmDelete = (): void => {
        setConfirmDelete(false);
        if (checkedIds.length === 0) {
            return;
        }
        void alertService.deleteBatch(checkedIds).then(() => {
            setCheckedIds([]);
            if (selectedId && checkedIds.includes(selectedId)) {
                setSelectedId(null);
            }
            reload();
        }).catch(err => {
            setError(resolveErrorText(err));
        });
    };

    const resolveCountLabel = (): string => {
        let text = tf('home.common.totalCount', {n: alerts.length});
        if (checkedIds.length > 0) {
            text += tf('home.common.selectedSuffix', {n: checkedIds.length});
        }
        return text;
    };

    useEffect(() => {
        setCheckedIds([]);
        setSelectedId(null);
        reload();
        return alertService.addAlertListChangeListener(() => {
            reload();
        });
    }, [alertService, jobDevice.id, link.job_indicator_id]);

    if (!isNil(selected)) {
        return (
            <Box className="tm-job-drawer-pane">
                <Box className="tm-job-drawer-scroll">
                    <AlertDetail alert={selected}/>
                </Box>
                <Box className="tm-job-drawer-actions">
                    <Button disableRipple variant="contained" className="tm-btn tm-btn-primary" onClick={() => setSelectedId(null)}>
                        {t('home.common.backToList')}
                    </Button>
                </Box>
            </Box>
        );
    }

    return (
        <Box className="tm-job-drawer-pane">
            <Box className="tm-job-drawer-scroll">
                <Typography className="tm-job-muted">
                    {resolveCountLabel()}
                </Typography>
                {loading && alerts.length === 0 ? <Typography className="tm-job-muted">{t('home.common.loading')}</Typography> : null}
                {error ? <Typography className="tm-job-drawer-error">{error}</Typography> : null}
                <AlertList alerts={alerts} selectedId={selectedId} checkedIds={checkedIds} indicator={indicator} onToggle={handleToggle} onToggleAll={handleToggleAll} onSelect={alert => setSelectedId(alert.id)} />
            </Box>
            <Box className="tm-job-drawer-actions">
                <Button disableRipple variant="contained" className="tm-btn tm-btn-danger" disabled={checkedIds.length === 0} onClick={() => setConfirmDelete(true)}>
                    {tf('home.common.deleteCount', {n: checkedIds.length})}
                </Button>
            </Box>
            <ConfirmDialog open={confirmDelete} title={t('home.job.alert.deleteTitle')} message={tf('home.job.alert.deleteMessage', {n: checkedIds.length})} confirmLabel={t('common.delete')} danger onCancel={() => setConfirmDelete(false)} onConfirm={handleConfirmDelete}/>
        </Box>
    );
}
