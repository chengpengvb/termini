import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { inTauri } from '@/common/util/TauriUtil.ts';
import { Box, Switch, Typography } from '@mui/material';
import { useEffect, useMemo, useRef, useState } from 'react';
import { ThemeRow } from '@/page/home/set/system/ThemeRow.tsx';
import type { ShellDescr } from '@/common/model/ShellDescr.ts';
import { clampInt } from '@/page/common/util/numberInputUtil.ts';
import { isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import type { SaveNotifyReq } from '@/common/model/req/ReqConfig.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { SelectField } from '@/page/common/components/input/SelectField.tsx';
import { getLanguage, setLanguage as applyLanguage } from '@/common/i18n/i18n.ts';
import type { SelectOption } from '@/page/common/components/input/SelectField.tsx';
import { resolveShellKind, resolveShellLabel } from '@/page/common/util/shellUtil.ts';
import { usePlatformMobile, usePlatformNativeMobile } from '@/common/context/usePlatformMobile';
import { useAccountService, useConfigService, useShellService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { CLOSE_ACTION, SHELL_CPU_PERCENT_DEFAULT, SHELL_CPU_PERCENT_MAX, SHELL_CPU_PERCENT_MIN, SYSTEM_LANGUAGE_OPTIONS, systemLanguageLabel, WEBHOOK_KIND } from '@/common/model/Config.ts';

const LANGUAGE_OPTIONS: SelectOption[] = SYSTEM_LANGUAGE_OPTIONS.map(value => ({
  value: String(value),
  label: systemLanguageLabel(value),
}));

export function GeneralSection() {
  const { t, tf } = useI18n();
  const mobile = usePlatformMobile();
  const shellService = useShellService();
  const configService = useConfigService();
  const accountService = useAccountService();
  const nativeMobile = usePlatformNativeMobile();
  const systemUtilService = useSystemUtilService();

  const notifySavedRef = useRef<string>('');
  const [shellId, setShellId] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [webhookUrl, setWebhookUrl] = useState<string>('');
  const [notifyEmail, setNotifyEmail] = useState<string>('');
  const [autoStart, setAutoStart] = useState<boolean>(false);
  const [shellList, setShellList] = useState<ShellDescr[]>([]);
  const [webhookExtra, setWebhookExtra] = useState<string>('');
  const [language, setLanguage] = useState<number>(getLanguage);
  const [notifyEnabled, setNotifyEnabled] = useState<boolean>(false);
  const [webhookEnabled, setWebhookEnabled] = useState<boolean>(false);
  const [openHereEnabled, setOpenHereEnabled] = useState<boolean>(true);
  const [closeAction, setCloseAction] = useState<number>(CLOSE_ACTION.ASK);
  const [notifyTrayEnabled, setNotifyTrayEnabled] = useState<boolean>(false);
  const [webhookKind, setWebhookKind] = useState<number>(WEBHOOK_KIND.DINGTALK);
  const [shellCpuPercent, setShellCpuPercent] = useState<string>(String(SHELL_CPU_PERCENT_DEFAULT));

  const shellOptions = useMemo<SelectOption[]>(() => {
    return shellList.map(item => ({
      value: item.shell_id,
      label: resolveShellLabel(resolveShellKind(item.shell_id)),
    }));
  }, [shellList, language]);

  const closeActionOptions = useMemo<SelectOption[]>(
    () => [
      { value: String(CLOSE_ACTION.ASK), label: t('set.general.closeActionAsk') },
      { value: String(CLOSE_ACTION.EXIT), label: t('set.general.closeActionExit') },
      { value: String(CLOSE_ACTION.TRAY), label: t('set.general.closeActionTray') },
    ],
    [t],
  );

  // const webhookKindOptions = useMemo<SelectOption[]>(
  //   () => [
  //     { value: String(WEBHOOK_KIND.DINGTALK), label: t('set.general.webhookKindDingtalk') },
  //     { value: String(WEBHOOK_KIND.WECOM), label: t('set.general.webhookKindWecom') },
  //     { value: String(WEBHOOK_KIND.FEISHU), label: t('set.general.webhookKindFeishu') },
  //     { value: String(WEBHOOK_KIND.TELEGRAM), label: t('set.general.webhookKindTelegram') },
  //   ],
  //   [t],
  // );

  const persist = (nextLanguage: number, nextShellId: string, nextCpuPercent: number): void => {
    setError(null);
    configService.saveSystem(nextLanguage, nextShellId === '' ? null : nextShellId, nextCpuPercent).catch(() => {
      setError(t('common.saveFailed'));
    });
  };

  const handleLanguageChange = (next: string): void => {
    const value = Number(next);
    setLanguage(value);
    applyLanguage(value);
    persist(value, shellId, Number(shellCpuPercent));
  };

  const handleShellChange = (next: string): void => {
    setShellId(next);
    persist(language, next, Number(shellCpuPercent));
  };

  const handleShellCpuBlur = (): void => {
    const value = clampInt(shellCpuPercent, SHELL_CPU_PERCENT_MIN, SHELL_CPU_PERCENT_MAX, SHELL_CPU_PERCENT_DEFAULT);
    setShellCpuPercent(String(value));
    persist(language, shellId, value);
  };

  const buildNotifyReq = (): SaveNotifyReq => ({
    email: notifyEmail === '' ? null : notifyEmail,
    enabled: notifyEnabled,
    trayEnabled: notifyTrayEnabled,
    webhookEnabled,
    webhookKind,
    webhookUrl: webhookUrl === '' ? null : webhookUrl,
    webhookExtra: webhookExtra === '' ? null : webhookExtra,
  });

  const persistNotify = (req: SaveNotifyReq): void => {
    const key = JSON.stringify(req);
    if (key === notifySavedRef.current) {
      return;
    }
    setError(null);
    configService.saveNotify(req).then(() => {
      notifySavedRef.current = key;
    }).catch(error => {
      setError(resolveErrorText(error));
    });
  };

  const handleNotifyEmailBlur = (): void => {
    persistNotify(buildNotifyReq());
  };

  const handleNotifyEnabledChange = (checked: boolean): void => {
    setNotifyEnabled(checked);
    persistNotify({ ...buildNotifyReq(), enabled: checked });
  };

  const handleNotifyTrayChange = (checked: boolean): void => {
    setNotifyTrayEnabled(checked);
    persistNotify({ ...buildNotifyReq(), trayEnabled: checked });
  };

  const handleAutoStartChange = (checked: boolean): void => {
    setAutoStart(checked);
    setError(null);
    systemUtilService.saveAutoStart(checked).catch(() => {
      setAutoStart(!checked);
      setError(t('common.saveFailed'));
    });
  };

  const handleOpenHereChange = (checked: boolean): void => {
    setOpenHereEnabled(checked);
    setError(null);
    systemUtilService.saveOpenHere(checked).catch(() => {
      setOpenHereEnabled(!checked);
      setError(t('common.saveFailed'));
    });
  };

  const handleCloseActionChange = (next: string): void => {
    const value = Number(next);
    setCloseAction(value);
    setError(null);
    configService.saveCloseAction(value).catch(() => {
      setError(t('common.saveFailed'));
    });
  };

  const renderError = (): ReactNode => {
    if (!isNotNil(error)) {
      return null;
    }
    return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
  };

  const resolveShellPlaceholder = (): string => {
    if (shellOptions.length === 0) {
      return t('set.general.shellEmpty');
    }
    return t('set.general.shellPlaceholder');
  };

  useEffect(() => {
    if (nativeMobile) {
      return;
    }
    shellService
      .discover()
      .then(setShellList)
      .catch(() => setShellList([]));
  }, [nativeMobile, shellService]);

  useEffect(() => {
    if (!inTauri() || mobile) {
      return;
    }
    systemUtilService
      .getAutoStart()
      .then(setAutoStart)
      .catch(() => setAutoStart(false));
  }, [systemUtilService, mobile]);

  useEffect(() => {
    configService
      .selectConfig()
      .then(config => {
        setLanguage(config?.system_language ?? getLanguage());
        setShellId(config?.local_shell_id ?? '');
        setOpenHereEnabled(config?.open_here_enabled !== false);
        setShellCpuPercent(String(config?.shell_cpu_percent || SHELL_CPU_PERCENT_DEFAULT));
        setCloseAction(config?.close_action ?? CLOSE_ACTION.ASK);
        const enabled = config?.notify_enabled ?? false;
        const email = isNotEmpty(config?.notify_email) ? config.notify_email : (accountService.getAccount()?.email ?? '');
        const trayEnabled = config?.notify_tray_enabled ?? false;
        const nextWebhookEnabled = config?.notify_webhook_enabled ?? false;
        const rawKind = config?.notify_webhook_kind ?? 0;
        const kind = rawKind === 0 ? WEBHOOK_KIND.DINGTALK : rawKind;
        const nextWebhookUrl = config?.notify_webhook_url ?? '';
        const nextWebhookExtra = config?.notify_webhook_extra ?? '';
        notifySavedRef.current = JSON.stringify({
          email: isNotEmpty(config?.notify_email) ? config.notify_email : null,
          enabled,
          trayEnabled,
          webhookEnabled: nextWebhookEnabled,
          webhookKind: kind,
          webhookUrl: nextWebhookUrl === '' ? null : nextWebhookUrl,
          webhookExtra: nextWebhookExtra === '' ? null : nextWebhookExtra,
        } satisfies SaveNotifyReq);
        setNotifyEnabled(enabled);
        setNotifyEmail(email ?? '');
        setNotifyTrayEnabled(trayEnabled);
        setWebhookEnabled(nextWebhookEnabled);
        setWebhookKind(kind);
        setWebhookUrl(nextWebhookUrl);
        setWebhookExtra(nextWebhookExtra);
      })
      .catch(() => {
        setLanguage(getLanguage());
      });
  }, [configService, accountService]);

  return (
    <Box className="tm-set-card">
      <Box className="tm-set-card-head">
        <Box className="tm-set-card-head-main">
          <Typography className="tm-set-card-title">{t('set.general.title')}</Typography>
          <Typography className="tm-set-card-desc">{t('set.general.desc')}</Typography>
        </Box>
        <Box className="tm-set-card-head-action">{renderError()}</Box>
      </Box>
      <Box className="tm-set-card-body">
        <Box className="tm-set-row">
          <Box className="tm-set-row-main">
            <Typography className="tm-set-row-label">{t('set.general.language')}</Typography>
            <Typography className="tm-set-row-hint">{t('set.general.languageHint')}</Typography>
          </Box>
          <Box className="tm-set-row-control">
            <SelectField value={String(language)} onChange={handleLanguageChange} options={LANGUAGE_OPTIONS} sheetTitle={t('set.general.language')} />
          </Box>
        </Box>
        <ThemeRow />
        {!nativeMobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.shell')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.shellHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <SelectField value={shellId} onChange={handleShellChange} options={shellOptions} placeholder={resolveShellPlaceholder()} sheetTitle={t('set.general.shell')} />
            </Box>
          </Box>
        ) : null}
        {inTauri() && !nativeMobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.openHere')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.openHereHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <Switch size="small" className="tm-set-switch" checked={openHereEnabled} onChange={e => handleOpenHereChange(e.target.checked)} />
            </Box>
          </Box>
        ) : null}
        {!nativeMobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.shellCpu')}</Typography>
              <Typography className="tm-set-row-hint">{tf('set.general.shellCpuHint', { min: SHELL_CPU_PERCENT_MIN, max: SHELL_CPU_PERCENT_MAX })}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <EditField value={shellCpuPercent} onChange={setShellCpuPercent} onBlur={handleShellCpuBlur} placeholder={String(SHELL_CPU_PERCENT_DEFAULT)} />
            </Box>
          </Box>
        ) : null}
        {!mobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.notifyEmail')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.notifyEmailHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <EditField value={notifyEmail} onChange={setNotifyEmail} onBlur={handleNotifyEmailBlur} placeholder={t('set.general.notifyEmailPlaceholder')} />
            </Box>
          </Box>
        ) : null}
        {!mobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.notifyEnabled')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.notifyEnabledHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <Switch size="small" className="tm-set-switch" checked={notifyEnabled} onChange={e => handleNotifyEnabledChange(e.target.checked)} />
            </Box>
          </Box>
        ) : null}
        {inTauri() && !mobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.notifyTray')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.notifyTrayHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <Switch size="small" className="tm-set-switch" checked={notifyTrayEnabled} onChange={e => handleNotifyTrayChange(e.target.checked)} />
            </Box>
          </Box>
        ) : null}
        {/*
        <Box className="tm-set-row">
          <Box className="tm-set-row-main">
            <Typography className="tm-set-row-label">{t('set.general.notifyWebhook')}</Typography>
            <Typography className="tm-set-row-hint">{t('set.general.notifyWebhookHint')}</Typography>
          </Box>
          <Box className="tm-set-row-control">
            <Switch size="small" className="tm-set-switch" checked={webhookEnabled} onChange={e => handleWebhookEnabledChange(e.target.checked)} />
          </Box>
        </Box>
        {webhookEnabled ? (
          <>
            <Box className="tm-set-row">
              <Box className="tm-set-row-main">
                <Typography className="tm-set-row-label">{t('set.general.notifyWebhookKind')}</Typography>
                <Typography className="tm-set-row-hint">{t('set.general.notifyWebhookKindHint')}</Typography>
              </Box>
              <Box className="tm-set-row-control">
                <SelectField value={String(webhookKind)} onChange={handleWebhookKindChange} options={webhookKindOptions} />
              </Box>
            </Box>
            <Box className="tm-set-row">
              <Box className="tm-set-row-main">
                <Typography className="tm-set-row-label">{t('set.general.notifyWebhookUrl')}</Typography>
                <Typography className="tm-set-row-hint">{t('set.general.notifyWebhookUrlHint')}</Typography>
              </Box>
              <Box className="tm-set-row-control">
                <EditField value={webhookUrl} onChange={setWebhookUrl} onBlur={handleWebhookUrlBlur} placeholder={t('set.general.notifyWebhookUrlPlaceholder')} />
              </Box>
            </Box>
            {webhookKind === WEBHOOK_KIND.TELEGRAM ? (
              <Box className="tm-set-row">
                <Box className="tm-set-row-main">
                  <Typography className="tm-set-row-label">{t('set.general.notifyWebhookExtra')}</Typography>
                  <Typography className="tm-set-row-hint">{t('set.general.notifyWebhookExtraHint')}</Typography>
                </Box>
                <Box className="tm-set-row-control">
                  <EditField value={webhookExtra} onChange={setWebhookExtra} onBlur={handleWebhookExtraBlur} placeholder={t('set.general.notifyWebhookExtraPlaceholder')} />
                </Box>
              </Box>
            ) : null}
          </>
        ) : null}
        */}
        {inTauri() && !mobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.autoStart')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.autoStartHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <Switch size="small" className="tm-set-switch" checked={autoStart} onChange={e => handleAutoStartChange(e.target.checked)} />
            </Box>
          </Box>
        ) : null}
        {inTauri() && !mobile ? (
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.general.closeAction')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.general.closeActionHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <SelectField value={String(closeAction)} onChange={handleCloseActionChange} options={closeActionOptions} />
            </Box>
          </Box>
        ) : null}
      </Box>
    </Box>
  );
}
