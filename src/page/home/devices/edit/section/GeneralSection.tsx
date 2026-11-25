import {useMemo} from 'react';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {Device} from '@/common/model/Device.ts';
import type {RespDevice} from '@/common/model/resp/RespDevice.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {GroupFields} from '@/page/home/devices/edit/section/shared/GroupFields.tsx';
import {DEVICE_OS_TYPE_OPTIONS, resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';

export function GeneralSection(props: {
    respDevice: RespDevice;
    onChange: (device: Device) => void;
    onGroupChange: (groupId: string) => void;
}) {
    const {t} = useI18n();
    const {respDevice, onChange, onGroupChange} = props;
    const device = respDevice.device;
    const osTypeOptions = useMemo(() => DEVICE_OS_TYPE_OPTIONS.map(option => {
        const osType = Number(option.value);
        return {
            value: option.value,
            label: t(`home.devices.osType.${option.value}`),
            icon: (
                <Box className={resolveDeviceOsIconChipClassName(osType)} style={resolveDeviceOsIconChipStyle(osType)}>
                    {resolveDeviceOsIcon(osType, 'tm-device-item-os-icon')}
                </Box>
            ),
        };
    }), [t]);

    const handleNameChange = (name: string): void => {
        onChange({...device, name});
    };

    const handleIpChange = (host: string): void => {
        onChange({...device, host});
    };

    const handleOsTypeChange = (osType: string): void => {
        onChange({...device, os_type: Number(osType)});
    };

    return (
        <Box className="tm-device-edit-fields">
            <Box className="tm-device-edit-row">
                <EditField label={t('home.common.name')} value={device.name} onChange={handleNameChange} placeholder={t('home.devices.namePlaceholder')}/>
                <EditField label={t('home.common.host')} value={device.host} onChange={handleIpChange} placeholder={t('home.devices.hostPlaceholder')}/>
            </Box>
            <Box className="tm-device-edit-row">
                <SelectField label={t('home.common.type')} value={String(device.os_type)} onChange={handleOsTypeChange} options={osTypeOptions} placeholder={t('home.devices.typePlaceholder')}/>
                <Box>
                    <GroupFields value={respDevice.group_id} onChange={onGroupChange}/>
                </Box>
            </Box>
        </Box>
    );
}
