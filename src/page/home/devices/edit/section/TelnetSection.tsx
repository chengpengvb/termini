import {Box} from '@mui/material';
import {EnvFields} from './shared/EnvFields.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ConnFields} from './shared/ConnFields.tsx';
import {TermFields} from './shared/TermFields.tsx';
import {BasicFields} from './shared/BasicFields.tsx';
import {isNil, isEmpty} from '@/common/util/StringUtil.ts';
import {TermTypesField} from './shared/TermTypesField.tsx';
import type {ConfigTelnet} from "@/common/model/ConfigTelnet.ts";
import type {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import {FoldSection} from '@/page/common/components/section/FoldSection.tsx';
import {KeyBindField} from '@/page/common/components/input/KeyBindField.tsx';
import {keyBindToSendText, sendTextToKeyBind, type KeyBind} from '@/common/util/KeyBindUtil.ts';
import {parseNonNegativeInt, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';
import { TELNET_DEFAULT_CMD_VERIFY_TIMEOUT, TELNET_DEFAULT_MAX_SESSIONS, TELNET_DEFAULT_MORE_PATTERN, TELNET_DEFAULT_MORE_RESPONSE, TELNET_DEFAULT_PASSWORD_PATTERN, TELNET_DEFAULT_PROMPT_ALT_PATTERN, TELNET_DEFAULT_PROMPT_PRI_PATTERN, TELNET_DEFAULT_USERNAME_PATTERN } from '@/common/const/DeviceConst.ts';

export function TelnetSection(props: { config?: ConfigTelnet | null; onChange: (config: ConfigTelnet) => void }) {
    const {t, tf} = useI18n();
    const {config, onChange} = props;

    if (!config) {
        return null;
    }

    const base = config.base;

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

    const handleTermChange = (term: string): void => {
        if (!base) {
            return;
        }
        updateBase({...base, term});
    };

    const handleUsernamePatternChange = (raw: string): void => {
        onChange({...config, username_pattern: parseOptionalText(raw)});
    };

    const handlePasswordPatternChange = (raw: string): void => {
        onChange({...config, password_pattern: parseOptionalText(raw)});
    };

    const handlePromptPriPatternChange = (raw: string): void => {
        onChange({...config, prompt_pri_pattern: parseOptionalText(raw)});
    };

    const handlePromptAltPatternChange = (raw: string): void => {
        onChange({...config, prompt_alt_pattern: parseOptionalText(raw)});
    };

    const handleMorePatternChange = (raw: string): void => {
        onChange({...config, more_pattern: parseOptionalText(raw)});
    };

    const resolveMoreResponseBind = (): KeyBind | null => {
        if (isNil(config.more_response)) {
            return null;
        }
        return sendTextToKeyBind(config.more_response);
    };

    const handleMoreResponseChange = (bind: KeyBind | null): void => {
        if (isNil(bind)) {
            onChange({...config, more_response: null});
            return;
        }
        onChange({...config, more_response: keyBindToSendText(bind)});
    };

    const handleDisablePagingCommandChange = (raw: string): void => {
        onChange({...config, disable_paging_command: parseOptionalText(raw)});
    };

    const handleCmdVerifyChange = (cmd_verify: boolean): void => {
        onChange({...config, cmd_verify});
    };

    const handleCmdVerifyTimeoutChange = (raw: string): void => {
        const parsed = parseNonNegativeInt(raw);
        onChange({...config, cmd_verify_timeout_seconds: parsed > 0 ? parsed : null});
    };

    const handleNvtEolChange = (nvt_eol: boolean): void => {
        onChange({...config, nvt_eol});
    };

    const handleMaxSessionsChange = (raw: string): void => {
        const parsed = parseNonNegativeInt(raw);
        onChange({...config, max_sessions: parsed > 0 ? parsed : TELNET_DEFAULT_MAX_SESSIONS});
    };

    const handleTerminalTypesChange = (terminal_types: string[] | null): void => {
        onChange({...config, terminal_types});
    };

    const handleEnvironmentChange = (environment: Record<string, string> | null): void => {
        onChange({...config, environment});
    };

    return (
        <Box className="tm-device-edit-fields">
            {base && <BasicFields connection={base} onChange={updateBase} portPlaceholder={t('home.devices.telnet.portPlaceholder')} usernamePlaceholder={t('home.devices.telnet.usernamePlaceholder')}/>}
            <FoldSection label={t('home.devices.telnet.moreParams')}>
                {base && <TermFields value={base.term} onChange={handleTermChange} placeholder={t('home.devices.telnet.termPlaceholder')}/>}
                <TermTypesField value={config.terminal_types} onChange={handleTerminalTypesChange}/>
                <SwitchField label={t('home.devices.telnet.nvtEol')} value={config.nvt_eol} onChange={handleNvtEolChange} description={t('home.devices.telnet.nvtEolDesc')}/>
                <EditField label={t('home.devices.telnet.maxSessions')} labelWidth={96} value={resolveDisplayNumber(config.max_sessions)} onChange={handleMaxSessionsChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_MAX_SESSIONS})}/>
                <EnvFields value={config.environment} onChange={handleEnvironmentChange}/>
                {base && <ConnFields connection={base} onChange={updateBase}/>}
            </FoldSection>
            <FoldSection label={t('home.devices.telnet.promptSection')}>
                <EditField label={t('home.devices.telnet.usernamePrompt')} labelWidth={96} value={resolveOptionalText(config.username_pattern)} onChange={handleUsernamePatternChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_USERNAME_PATTERN})}/>
                <EditField label={t('home.devices.telnet.passwordPrompt')} labelWidth={96} value={resolveOptionalText(config.password_pattern)} onChange={handlePasswordPatternChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_PASSWORD_PATTERN})}/>
                <EditField label={t('home.devices.telnet.primaryPrompt')} labelWidth={96} value={resolveOptionalText(config.prompt_pri_pattern)} onChange={handlePromptPriPatternChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_PROMPT_PRI_PATTERN})}/>
                <EditField label={t('home.devices.telnet.altPrompt')} labelWidth={96} value={resolveOptionalText(config.prompt_alt_pattern)} onChange={handlePromptAltPatternChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_PROMPT_ALT_PATTERN})}/>
                <EditField label={t('home.devices.telnet.morePrompt')} labelWidth={96} value={resolveOptionalText(config.more_pattern)} onChange={handleMorePatternChange} placeholder={tf('home.devices.defaultValue', {value: TELNET_DEFAULT_MORE_PATTERN})}/>
                <KeyBindField label={t('home.devices.telnet.moreResponse')} labelWidth={96} value={resolveMoreResponseBind()} defaultValue={sendTextToKeyBind(TELNET_DEFAULT_MORE_RESPONSE)} onChange={handleMoreResponseChange}/>
                <EditField label={t('home.devices.telnet.disablePaging')} labelWidth={96} value={resolveOptionalText(config.disable_paging_command)} onChange={handleDisablePagingCommandChange} placeholder={t('home.devices.telnet.disablePagingPlaceholder')}/>
            </FoldSection>
            <FoldSection label={t('home.devices.telnet.scriptSection')}>
                <SwitchField label={t('home.devices.telnet.cmdVerify')} value={config.cmd_verify ?? true} onChange={handleCmdVerifyChange} description={t('home.devices.telnet.cmdVerifyDesc')}/>
                <EditField label={t('home.devices.telnet.cmdVerifyTimeout')} labelWidth={96} value={resolveDisplayNumber(config.cmd_verify_timeout_seconds)} onChange={handleCmdVerifyTimeoutChange} placeholder={tf('home.devices.defaultValueSeconds', {value: TELNET_DEFAULT_CMD_VERIFY_TIMEOUT})}/>
            </FoldSection>
        </Box>
    );
}
