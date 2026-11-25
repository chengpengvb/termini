import {useMemo} from 'react';
import {Box} from '@mui/material';
import {FolderOpen} from '@mui/icons-material';
import {open} from '@tauri-apps/plugin-dialog';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ConnFields} from './shared/ConnFields.tsx';
import {BasicFields} from './shared/BasicFields.tsx';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import type {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {RDP_SECURITY, type ConfigRdp} from "@/common/model/ConfigRdp.ts";
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import {FoldSection} from '@/page/common/components/section/FoldSection.tsx';
import {SelectField, type SelectOption} from '@/page/common/components/input/SelectField.tsx';
import {parseNonNegativeInt, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';
import {RDP_DEFAULT_COLOR_DEPTH, rdpKeyboardLayoutOptions, rdpPerformanceOptions} from '@/common/const/DeviceConst.ts';

export function RdpSection(props: { config?: ConfigRdp | null; onChange: (config: ConfigRdp) => void }) {
    const {t, tf} = useI18n();
    const {config, onChange} = props;

    if (!config) {
        return null;
    }

    const base = config.base;
    const securityOptions = useMemo<SelectOption[]>(() => ([
        {value: String(RDP_SECURITY.AUTO), label: t('home.devices.rdp.securityAuto')},
        {value: String(RDP_SECURITY.TLS), label: 'TLS'},
        {value: String(RDP_SECURITY.NLA), label: 'NLA (CredSSP)'},
    ]), [t]);
    const colorDepthOptions = useMemo<SelectOption[]>(() => ([
        {value: '32', label: t('home.devices.rdp.color32')},
        {value: '24', label: t('home.devices.rdp.color24')},
        {value: '16', label: t('home.devices.rdp.color16')},
    ]), [t]);

    const updateBase = (next: ConfigTermBase): void => {
        onChange({...config, base: next});
    };

    const resolveOptionalText = (text: string | null | undefined): string | null => {
        if (isNil(text)) {
            return null;
        }
        return text;
    };

    const parseOptionalText = (raw: string): string | null => {
        if (isEmpty(raw)) {
            return null;
        }
        return raw;
    };

    const parseOptionalEdge = (raw: string): number | null => {
        const parsed = parseNonNegativeInt(raw);
        if (parsed <= 0) {
            return null;
        }
        return parsed;
    };

    const handleDomainChange = (raw: string): void => {
        onChange({...config, domain: parseOptionalText(raw)});
    };

    const handleSecurityChange = (raw: string): void => {
        onChange({...config, security: Number(raw)});
    };

    const handleColorDepthChange = (raw: string): void => {
        onChange({...config, color_depth: Number(raw)});
    };

    const handleIgnoreCertChange = (ignore_cert: boolean): void => {
        onChange({...config, ignore_cert});
    };

    const handleConsoleSessionChange = (console_session: boolean): void => {
        onChange({...config, console_session});
    };

    const handleKeyboardLayoutChange = (raw: string): void => {
        onChange({...config, keyboard_layout: Number(raw)});
    };

    const resolvePerformanceValue = (): string => {
        if (isNil(config.performance_flags)) {
            return '';
        }
        return String(config.performance_flags);
    };

    const handleDesktopWidthChange = (raw: string): void => {
        onChange({...config, desktop_width: parseOptionalEdge(raw)});
    };

    const handleDesktopHeightChange = (raw: string): void => {
        onChange({...config, desktop_height: parseOptionalEdge(raw)});
    };

    const handlePerformanceFlagsChange = (raw: string): void => {
        if (isEmpty(raw)) {
            onChange({...config, performance_flags: null});
            return;
        }
        onChange({...config, performance_flags: parseNonNegativeInt(raw)});
    };

    const handleDrivePathChange = (raw: string): void => {
        onChange({...config, drive_path: parseOptionalText(raw)});
    };

    const handlePickDrive = (): void => {
        void open({directory: true, multiple: false}).then(selected => {
            if (selected === null) {
                return;
            }
            onChange({...config, drive_path: selected});
        });
    };

    const handleAadClientIdChange = (raw: string): void => {
        onChange({...config, aad_client_id: parseOptionalText(raw)});
    };

    const handleAadTenantChange = (raw: string): void => {
        onChange({...config, aad_tenant: parseOptionalText(raw)});
    };

    return (
        <Box className="tm-device-edit-fields">
            {base && <BasicFields connection={base} onChange={updateBase} portPlaceholder={t('home.devices.rdp.portPlaceholder')} usernamePlaceholder={t('home.devices.rdp.usernamePlaceholder')}/>}
            <EditField label={t('home.devices.rdp.domain')} value={resolveOptionalText(config.domain)} onChange={handleDomainChange} placeholder={t('home.devices.rdp.domainPlaceholder')}/>
            <FoldSection label={t('home.devices.rdp.moreParams')}>
                <SelectField label={t('home.devices.rdp.security')} value={String(config.security)} onChange={handleSecurityChange} options={securityOptions}/>
                <SelectField label={t('home.devices.rdp.colorDepth')} value={String(config.color_depth)} onChange={handleColorDepthChange} options={colorDepthOptions} placeholder={tf('home.devices.rdp.colorDepthDefault', {n: RDP_DEFAULT_COLOR_DEPTH})}/>
                <SwitchField label={t('home.devices.rdp.ignoreCert')} value={config.ignore_cert} onChange={handleIgnoreCertChange} description={t('home.devices.rdp.ignoreCertDesc')}/>
                <SwitchField label={t('home.devices.rdp.consoleSession')} value={config.console_session} onChange={handleConsoleSessionChange} description={t('home.devices.rdp.consoleSessionDesc')}/>
                <SelectField label={t('home.devices.rdp.keyboardLayout')} labelWidth={96} value={String(config.keyboard_layout)} onChange={handleKeyboardLayoutChange} options={rdpKeyboardLayoutOptions()} searchable searchPlaceholder={t('home.devices.rdp.keyboardSearch')}/>
                {base && <ConnFields connection={base} onChange={updateBase}/>}
            </FoldSection>
            <FoldSection label={t('home.devices.rdp.desktopSection')}>
                <EditField label={t('home.devices.rdp.desktopWidth')} labelWidth={96} value={resolveDisplayNumber(config.desktop_width)} onChange={handleDesktopWidthChange} placeholder={t('home.devices.rdp.desktopPlaceholder')}/>
                <EditField label={t('home.devices.rdp.desktopHeight')} labelWidth={96} value={resolveDisplayNumber(config.desktop_height)} onChange={handleDesktopHeightChange} placeholder={t('home.devices.rdp.desktopPlaceholder')}/>
                <SelectField label={t('home.devices.rdp.experience')} labelWidth={96} value={resolvePerformanceValue()} onChange={handlePerformanceFlagsChange} options={rdpPerformanceOptions()} placeholder={t('home.common.auto')}/>
            </FoldSection>
            <FoldSection label={t('home.devices.rdp.redirectSection')}>
                <EditField label={t('home.devices.rdp.drivePath')} labelWidth={96} value={resolveOptionalText(config.drive_path)} onChange={handleDrivePathChange} placeholder={t('home.devices.rdp.drivePlaceholder')} actionLabel={t('home.devices.rdp.pickFolder')} actionIcon={<FolderOpen/>} onAction={handlePickDrive}/>
            </FoldSection>
            <FoldSection label={t('home.devices.rdp.aadSection')}>
                <EditField label={t('home.devices.rdp.aadClientId')} labelWidth={96} value={resolveOptionalText(config.aad_client_id)} onChange={handleAadClientIdChange} placeholder={t('home.devices.rdp.aadClientPlaceholder')}/>
                <EditField label={t('home.devices.rdp.aadTenant')} labelWidth={96} value={resolveOptionalText(config.aad_tenant)} onChange={handleAadTenantChange} placeholder={t('home.devices.rdp.aadTenantPlaceholder')}/>
            </FoldSection>
        </Box>
    );
}
