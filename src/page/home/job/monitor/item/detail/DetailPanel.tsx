import {DetailTab} from './DetailTab.tsx';
import {useEffect, useState} from 'react';
import {AlertTab} from '../alert/AlertTab.tsx';
import {ArrowForward} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {isDraftLink} from '@/page/common/util/jobUtil.ts';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {useJobDeviceService} from '@/common/context/useAppHooks.tsx';
import type {JobDevice, JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {JOB_DEVICE_DRAWER_TAB, type JobDeviceDrawerTab} from '@/page/common/model/JobModel';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';

export interface DetailPanelProps {
    open: boolean;
    onClose: () => void;
    onDeleted: () => void;
    jobDevice: JobDevice | null;
    link: JobDevice2Indicator | null;
    onSaved: (link: JobDevice2Indicator) => void;
}

export function DetailPanel(props: DetailPanelProps) {
    const {t} = useI18n();
    const device = props.jobDevice?.device;
    const deviceName = device?.name ?? '';
    const jobDeviceService = useJobDeviceService();
    const {open, jobDevice, link, onClose, onSaved, onDeleted} = props;
    const name = link?.job_indicator?.name || link?.job_indicator_id || '';
    const draft = isDraftLink(link);

    const handleTabChange = (next: JobDeviceDrawerTab): void => {
        setTab(next);
        jobDeviceService.setDrawerTab(next);
    };

    const [tab, setTab] = useState<JobDeviceDrawerTab>(() => jobDeviceService.getDrawerTab());

    useEffect(() => {
        setTab(jobDeviceService.getDrawerTab());
    }, [jobDeviceService, link?.id]);

    if (!open || isNil(link) || isNil(jobDevice)) {
        return null;
    }

    return (
        <Box className="tm-job-drawer">
            <Box className="tm-job-drawer-head">
                <Box className="tm-job-drawer-head-main">
                    <Box className="tm-job-drawer-title-row">
                        {device ? (
                            <Box className={resolveDeviceOsIconChipClassName(device.os_type)} style={resolveDeviceOsIconChipStyle(device.os_type)}>
                                {resolveDeviceOsIcon(device.os_type, 'tm-device-item-os-icon')}
                            </Box>
                        ) : null}
                        {!isEmpty(deviceName) ? <Typography className="tm-job-drawer-device">{deviceName}</Typography> : null}
                    </Box>
                    <Typography className="tm-job-drawer-title">{isEmpty(name) ? t('home.job.metricDraft') : name}</Typography>
                </Box>
                <ButtonBase disableRipple className="tm-job-icon-btn" onClick={onClose} aria-label={t('home.common.fold')}>
                    <ArrowForward fontSize="small"/>
                </ButtonBase>
            </Box>
            {!draft ? (
                <Box className="tm-job-drawer-tabs">
                    <Box className="tm-job-tabs">
                        <ButtonBase disableRipple className={tab === JOB_DEVICE_DRAWER_TAB.DETAIL ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_DEVICE_DRAWER_TAB.DETAIL)}>
                            <Typography component="span">{t('home.job.tabDetail')}</Typography>
                        </ButtonBase>
                        <ButtonBase disableRipple className={tab === JOB_DEVICE_DRAWER_TAB.ALERT ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_DEVICE_DRAWER_TAB.ALERT)}>
                            <Typography component="span">{t('home.job.tabAlerts')}</Typography>
                        </ButtonBase>
                    </Box>
                </Box>
            ) : null}
            <Box className="tm-job-drawer-body">
                {draft || tab === JOB_DEVICE_DRAWER_TAB.DETAIL ? (
                    <DetailTab jobDevice={jobDevice} link={link} onSaved={onSaved} onDeleted={onDeleted}/>
                ) : (
                    <AlertTab jobDevice={jobDevice} link={link}/>
                )}
            </Box>
        </Box>
    );
}
