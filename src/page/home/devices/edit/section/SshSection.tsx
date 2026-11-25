import {useMemo} from 'react';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ConnFields} from './shared/ConnFields.tsx';
import {TermFields} from './shared/TermFields.tsx';
import {BasicFields} from './shared/BasicFields.tsx';
import {KeyPairFields} from './shared/KeyPairFields.tsx';
import type {ConfigSsh} from "@/common/model/ConfigSsh.ts";
import {isEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import type {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import {FoldSection} from '@/page/common/components/section/FoldSection.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';
import {parseNonNegativeInt, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';

export interface SshJumpOption {
    id: string;
    label: string;
    os_type: number;
}

export function SshSection(props: { config?: ConfigSsh | null; onChange: (config: ConfigSsh) => void; jumpOptions?: SshJumpOption[] }) {
    const {t} = useI18n();
    const {config, onChange, jumpOptions} = props;

    if (!config) {
        return null;
    }

    const base = config.base;
    const authMethodOptions = useMemo(() => ([
        { value: '', label: t('home.devices.ssh.authAuto') },
        { value: '2', label: t('home.devices.ssh.authPublicKey') },
        { value: '1', label: t('home.devices.ssh.authPassword') },
        { value: '3', label: t('home.devices.ssh.authKeyboard') },
    ]), [t]);
    const jumpSelectOptions = useMemo(() => {
        const none = {value: '', label: t('home.devices.ssh.jumpNone')};
        if (!isNotNil(jumpOptions)) {
            return [none];
        }
        return [none, ...jumpOptions.map(item => ({
            value: item.id,
            label: item.label,
            icon: (
                <Box className={resolveDeviceOsIconChipClassName(item.os_type)} style={resolveDeviceOsIconChipStyle(item.os_type)}>
                    {resolveDeviceOsIcon(item.os_type, 'tm-device-item-os-icon')}
                </Box>
            ),
        }))];
    }, [jumpOptions, t]);

    const resolveOptionalText = (text: string | null | undefined): string | null => {
        if (!isNotNil(text)) {
            return null;
        }
        if (isEmpty(text)) {
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

    const updateBase = (next: ConfigTermBase): void => {
        onChange({...config, base: next});
    };

    const handleTermChange = (term: string): void => {
        if (!base) {
            return;
        }
        updateBase({...base, term});
    };

    const handleUseSshAgentChange = (use_ssh_agent: boolean): void => {
        onChange({...config, use_ssh_agent});
    };

    const handleUseGssapiChange = (use_gssapi: boolean): void => {
        onChange({...config, use_gssapi});
    };

    const handleShareSessionChange = (share_session: boolean): void => {
        onChange({...config, share_session});
    };

    const handleJumpDeviceChange = (raw: string): void => {
        onChange({...config, jump_device_id: isEmpty(raw) ? null : raw});
    };

    const resolveJumpDeviceValue = (): string => {
        if (!isNotNil(config.jump_device_id) || isEmpty(config.jump_device_id)) {
            return '';
        }
        return config.jump_device_id;
    };

    const handleKeyPairChange = (config_key_pair_id: string): void => {
        onChange({...config, config_key_pair_id});
    };

    const handlePassphraseChange = (raw: string): void => {
        onChange({...config, passphrase: parseOptionalText(raw)});
    };

    const handleAgentKeyIdChange = (raw: string): void => {
        onChange({...config, agent_key_id: parseOptionalText(raw)});
    };

    const handleShellIdleTimeoutSecondsChange = (raw: string): void => {
        onChange({...config, shell_idle_timeout_seconds: parseNonNegativeInt(raw)});
    };

    const handlePreferredAuthMethodChange = (raw: string): void => {
        if (isEmpty(raw)) {
            onChange({...config, preferred_auth_method: null});
            return;
        }
        const parsed = Number(raw);
        if (!Number.isFinite(parsed)) {
            onChange({...config, preferred_auth_method: null});
            return;
        }
        const code = Math.trunc(parsed);
        if (code === 1 || code === 2 || code === 3) {
            onChange({...config, preferred_auth_method: code});
            return;
        }
        onChange({...config, preferred_auth_method: null});
    };

    const resolvePreferredAuthMethodValue = (): string | null => {
        const v = config.preferred_auth_method;
        if (!isNotNil(v)) {
            return null;
        }
        return String(v);
    };

    const resolveKeyboardInteractiveAnswersValue = (): string | null => {
        const answers = config.keyboard_interactive_answers;
        if (!isNotNil(answers)) {
            return null;
        }
        if (answers.length === 0) {
            return null;
        }
        return answers.join('\n');
    };

    const handleKeyboardInteractiveAnswersChange = (raw: string): void => {
        if (isEmpty(raw)) {
            onChange({...config, keyboard_interactive_answers: null});
            return;
        }
        const answers = raw.split('\n').map(s => s.trim()).filter(s => !isEmpty(s));
        if (answers.length === 0) {
            onChange({...config, keyboard_interactive_answers: null});
            return;
        }
        onChange({...config, keyboard_interactive_answers: answers});
    };

    return (
        <Box className="tm-device-edit-fields">
            {base && <BasicFields connection={base} onChange={updateBase} portPlaceholder={t('home.devices.ssh.portPlaceholder')} usernamePlaceholder={t('home.devices.ssh.usernamePlaceholder')}/>}
            <KeyPairFields value={config.config_key_pair_id} onChange={handleKeyPairChange}/>
            <SelectField label={t('home.devices.ssh.jumpDevice')} value={resolveJumpDeviceValue()} onChange={handleJumpDeviceChange} placeholder={t('home.devices.ssh.jumpNone')} options={jumpSelectOptions} searchable/>
            <FoldSection label={t('home.devices.ssh.moreParams')}>
                {base && <TermFields value={base.term} onChange={handleTermChange} placeholder={t('home.devices.ssh.termPlaceholder')}/>}
                <SwitchField label={t('home.devices.ssh.shareSession')} value={config.share_session} onChange={handleShareSessionChange} description={t('home.devices.ssh.shareSessionDesc')}/>
                <SwitchField label={t('home.devices.ssh.useAgent')} value={config.use_ssh_agent} onChange={handleUseSshAgentChange} description={t('home.devices.ssh.useAgentDesc')}/>
                <SwitchField label={t('home.devices.ssh.useGssapi')} value={config.use_gssapi} onChange={handleUseGssapiChange} description={t('home.devices.ssh.useGssapiDesc')}/>
                <EditField label={t('home.devices.ssh.passphrase')} value={resolveOptionalText(config.passphrase)} onChange={handlePassphraseChange} placeholder={t('home.devices.ssh.passphrasePlaceholder')} password/>
                <EditField label={t('home.devices.ssh.agentKey')} value={resolveOptionalText(config.agent_key_id)} onChange={handleAgentKeyIdChange} placeholder={t('home.devices.ssh.agentKeyPlaceholder')}/>
                <EditField label={t('home.devices.ssh.idleClose')} value={resolveDisplayNumber(config.shell_idle_timeout_seconds)} onChange={handleShellIdleTimeoutSecondsChange} placeholder={t('home.devices.ssh.idleClosePlaceholder')} />
                <SelectField label={t('home.devices.ssh.authMethod')} value={resolvePreferredAuthMethodValue()} onChange={handlePreferredAuthMethodChange} placeholder={t('home.devices.ssh.authAuto')} options={authMethodOptions} />
                <TextareaField label={t('home.devices.ssh.interactiveAnswers')} value={resolveKeyboardInteractiveAnswersValue()} onChange={handleKeyboardInteractiveAnswersChange} placeholder={t('home.devices.ssh.interactivePlaceholder')} minRows={2} maxRows={6}/>
                {base && <ConnFields connection={base} onChange={updateBase}/>}
            </FoldSection>
        </Box>
    );
}
