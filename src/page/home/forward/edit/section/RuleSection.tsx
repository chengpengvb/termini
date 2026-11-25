import {Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {FORWARD_KIND, PortForward} from '@/common/model/PortForward.ts';
import {resolveForwardKindHint} from '@/page/common/util/forwardUtil.ts';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import type {SelectOption} from '@/page/common/components/input/SelectField.tsx';
import {parsePort, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';

export function RuleSection(props: { value: PortForward; onChange: (value: PortForward) => void; deviceOptions: SelectOption[] }) {
    const {t} = useI18n();
    const {value, onChange, deviceOptions} = props;
    const showTarget = value.kind !== FORWARD_KIND.DYNAMIC;
    const bindPortLabel = value.kind === FORWARD_KIND.REMOTE ? t('home.forward.bindPortRemote') : t('home.forward.bindPortLocal');

    const handleNameChange = (name: string): void => {
        onChange({...value, name});
    };

    const handleBindPortChange = (raw: string): void => {
        onChange({...value, bind_port: parsePort(raw)});
    };

    const handleBindAddressChange = (bind_address: string): void => {
        onChange({...value, bind_address});
    };

    const handleDeviceChange = (device_id: string): void => {
        onChange({...value, device_id});
    };

    const handleTargetHostChange = (target_host: string): void => {
        onChange({...value, target_host});
    };

    const handleTargetPortChange = (raw: string): void => {
        onChange({...value, target_port: parsePort(raw)});
    };

    const handleEnabledChange = (enabled: boolean): void => {
        onChange({...value, enabled});
    };

    return (
        <>
            <Typography sx={{fontSize: 12, color: 'var(--tm-text-tertiary)', lineHeight: 1.5}}>{resolveForwardKindHint(value.kind)}</Typography>
            <EditField label={t('home.common.name')} value={value.name} onChange={handleNameChange} placeholder={t('home.forward.namePlaceholder')}/>
            <EditField label={bindPortLabel} value={resolveDisplayNumber(value.bind_port)} onChange={handleBindPortChange} placeholder={t('home.forward.targetPortPlaceholder')}/>
            <EditField label={t('home.forward.bindAddress')} value={value.bind_address} onChange={handleBindAddressChange} placeholder={t('home.forward.bindAddressPlaceholder')}/>
            <SelectField label={t('home.common.host')} value={value.device_id} onChange={handleDeviceChange} options={deviceOptions} placeholder={t('home.forward.devicePlaceholder')} searchable searchPlaceholder={t('home.forward.deviceSearch')}/>
            {showTarget ? (
                <EditField label={t('home.forward.targetAddress')} value={value.target_host} onChange={handleTargetHostChange} placeholder={t('home.forward.targetHostPlaceholder')}/>
            ) : null}
            {showTarget ? (
                <EditField label={t('home.forward.targetPort')} value={resolveDisplayNumber(value.target_port)} onChange={handleTargetPortChange} placeholder={t('home.forward.targetPortPlaceholder')}/>
            ) : null}
            <SwitchField label={t('home.forward.enabled')} value={value.enabled} onChange={handleEnabledChange}/>
        </>
    );
}
