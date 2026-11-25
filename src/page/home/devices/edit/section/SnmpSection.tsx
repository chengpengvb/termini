import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ConfigSnmp} from '@/common/model/ConfigSnmp.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import {parseNonNegativeInt, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';
import {SNMP_DEFAULT_PORT, SNMP_DEFAULT_RETRIES, SNMP_DEFAULT_TIMEOUT_SECONDS} from '@/common/const/DeviceConst.ts';

const VERSION_OPTIONS = [
    {value: '1', label: 'v1'},
    {value: '2', label: 'v2c'},
];

export function SnmpSection(props: { config?: ConfigSnmp | null; onChange: (config: ConfigSnmp) => void }) {
    const {t, tf} = useI18n();
    const {config, onChange} = props;

    if (!config) {
        return null;
    }

    const handleEnabledChange = (enabled: boolean): void => {
        onChange({...config, enabled});
    };

    const handleVersionChange = (raw: string): void => {
        onChange({...config, version: Number(raw)});
    };

    const handleCommunityChange = (community: string): void => {
        onChange({...config, community});
    };

    const handlePortChange = (raw: string): void => {
        const parsed = parseNonNegativeInt(raw);
        onChange({...config, port: parsed > 0 ? parsed : SNMP_DEFAULT_PORT});
    };

    const handleTimeoutChange = (raw: string): void => {
        const parsed = parseNonNegativeInt(raw);
        onChange({...config, timeout_seconds: parsed > 0 ? parsed : SNMP_DEFAULT_TIMEOUT_SECONDS});
    };

    const handleRetriesChange = (raw: string): void => {
        onChange({...config, retries: parseNonNegativeInt(raw)});
    };

    return (
        <Box className="tm-device-edit-fields">
            <SwitchField label={t('home.devices.snmp.enabled')} value={config.enabled} onChange={handleEnabledChange} description={t('home.devices.snmp.enabledDesc')}/>
            <SelectField label={t('home.devices.snmp.version')} labelWidth={96} value={String(config.version)} onChange={handleVersionChange} options={VERSION_OPTIONS}/>
            <EditField label={t('home.devices.snmp.community')} labelWidth={96} value={config.community} onChange={handleCommunityChange} placeholder={t('home.devices.snmp.communityPlaceholder')}/>
            <EditField label={t('home.devices.snmp.port')} labelWidth={96} value={resolveDisplayNumber(config.port)} onChange={handlePortChange} placeholder={tf('home.devices.defaultValue', {value: SNMP_DEFAULT_PORT})}/>
            <EditField label={t('home.devices.snmp.timeout')} labelWidth={96} value={resolveDisplayNumber(config.timeout_seconds)} onChange={handleTimeoutChange} placeholder={tf('home.devices.defaultValueSeconds', {value: SNMP_DEFAULT_TIMEOUT_SECONDS})}/>
            <EditField label={t('home.devices.snmp.retries')} labelWidth={96} value={resolveDisplayNumber(config.retries)} onChange={handleRetriesChange} placeholder={tf('home.devices.defaultValue', {value: SNMP_DEFAULT_RETRIES})}/>
        </Box>
    );
}
