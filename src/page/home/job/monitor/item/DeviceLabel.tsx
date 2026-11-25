import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {WarningAmberRounded} from '@mui/icons-material';
import type {JobDevice} from '@/common/model/JobDevice.ts';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';

export interface DeviceLabelProps {
    firing: boolean;
    jobDevice: JobDevice;
}

export function DeviceLabel(props: DeviceLabelProps) {
    const {t, tf} = useI18n();
    const {jobDevice, firing} = props;
    const device = jobDevice.device;
    const indicators = jobDevice.indicators ?? [];

    const resolveOsLabel = (osType: number): string => t(`home.devices.osType.${osType}`);

    return (
        <Box className="tm-job-device-head-main">
            <Box className="tm-job-device-title-row">
                {device ? (
                    <Box className={resolveDeviceOsIconChipClassName(device.os_type)} style={resolveDeviceOsIconChipStyle(device.os_type)}>
                        {resolveDeviceOsIcon(device.os_type, 'tm-device-item-os-icon')}
                    </Box>
                ) : null}
                {firing ? <WarningAmberRounded className="tm-job-device-alert" titleAccess={t('home.job.deviceAlert')}/> : null}
                {device?.name ? <Typography className="tm-job-device-name">{device.name}</Typography> : null}
            </Box>
            <Typography className="tm-job-device-meta">
                {device?.host}
                {device ? ` · ${resolveOsLabel(device.os_type)}` : ''}
                {jobDevice.enabled ? '' : t('home.job.devicePaused')}
                {indicators.length > 0 ? tf('home.job.deviceMetricCount', {n: indicators.length}) : ''}
            </Typography>
        </Box>
    );
}
