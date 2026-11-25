import { useEffect, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { ChevronRight } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { FeedbackDialog } from '@/page/home/set/about/FeedbackDialog.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile.ts';
import type { AboutInfo, AboutSiteKind } from '@/common/model/resp/RespAppVersion.ts';
import { useAccountService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';

export function About() {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const accountService = useAccountService();
  const systemUtilService = useSystemUtilService();
  const [exporting, setExporting] = useState(false);
  const [feedbackOpen, setFeedbackOpen] = useState(false);
  const [about, setAbout] = useState<AboutInfo | null>(null);
  const [feedbackNote, setFeedbackNote] = useState<string | null>(null);
  const siteReady = about !== null && about.websiteUrl != null && about.websiteUrl.length > 0;

  function handleOpenSite(kind: AboutSiteKind): void {
    void accountService.openSite(kind);
  }

  function handleOpenPortal(): void {
    void accountService.openPortal();
  }

  function handleOpenFeedback(): void {
    setFeedbackNote(null);
    setFeedbackOpen(true);
  }

  function handleFeedbackSent(): void {
    setFeedbackOpen(false);
    setFeedbackNote(t('set.about.feedbackSent'));
  }

  function handleExportDiagnostics(): void {
    if (exporting) {
      return;
    }
    const stamp = new Date().toISOString().slice(0, 19).replace(/[-:T]/g, '');
    save({ defaultPath: `termini-diagnostics-${stamp}.zip`, filters: [{ name: 'Zip', extensions: ['zip'] }] })
      .then(target => {
        if (target === null) {
          return;
        }
        setExporting(true);
        setFeedbackNote(t('set.about.diagnosticsExporting'));
        return systemUtilService.exportDiagnostics(target).then(result => {
          setFeedbackNote(tf('set.about.diagnosticsDone', { path: result.path, files: result.files }));
        });
      })
      .catch((err: unknown) => {
        setFeedbackNote(tf('set.about.diagnosticsFailed', { error: resolveErrorText(err) }));
      })
      .finally(() => {
        setExporting(false);
      });
  }

  useEffect(() => {
    accountService
      .about()
      .then(setAbout)
      .catch(() => {
        setAbout(null);
      });
  }, [accountService]);

  return (
    <Box className="tm-set-content">
      <Box className="tm-set-content-header">
        <Typography className="tm-set-content-title">{t('set.about.title')}</Typography>
      </Box>

      <Box className="tm-about-version-card">
        <Box className="tm-about-version-main">
          <Typography className="tm-about-product">Termini</Typography>
          <Typography className="tm-about-version">
            {t('set.about.version')} {about === null ? '-' : about.version}
          </Typography>
        </Box>
      </Box>

      <Box className="tm-about-links">
        <ButtonBase disableRipple className="tm-about-link" disabled={!siteReady} onClick={() => handleOpenSite('website')}>
          <Typography className="tm-about-link-label">{t('set.about.website')}</Typography>
          <ChevronRight className="tm-about-link-chevron" />
        </ButtonBase>
        <ButtonBase disableRipple className="tm-about-link" onClick={handleOpenPortal}>
          <Typography className="tm-about-link-label">{t('set.about.portal')}</Typography>
          <ChevronRight className="tm-about-link-chevron" />
        </ButtonBase>
        <ButtonBase disableRipple className="tm-about-link" disabled={!siteReady} onClick={() => handleOpenSite('privacy')}>
          <Typography className="tm-about-link-label">{t('set.about.privacy')}</Typography>
          <ChevronRight className="tm-about-link-chevron" />
        </ButtonBase>
        <ButtonBase disableRipple className="tm-about-link" disabled={!siteReady} onClick={() => handleOpenSite('terms')}>
          <Typography className="tm-about-link-label">{t('set.about.terms')}</Typography>
          <ChevronRight className="tm-about-link-chevron" />
        </ButtonBase>
        <ButtonBase disableRipple className="tm-about-link" onClick={handleOpenFeedback}>
          <Typography className="tm-about-link-label">{t('set.about.feedback')}</Typography>
          <ChevronRight className="tm-about-link-chevron" />
        </ButtonBase>
        {mobileViewport ? null : (
          <ButtonBase disableRipple className="tm-about-link" disabled={exporting} title={t('set.about.diagnosticsHint')} onClick={handleExportDiagnostics}>
            <Typography className="tm-about-link-label">{exporting ? t('set.about.diagnosticsExporting') : t('set.about.diagnostics')}</Typography>
            <ChevronRight className="tm-about-link-chevron" />
          </ButtonBase>
        )}
      </Box>
      {feedbackNote !== null ? <Typography className="tm-about-check-note">{feedbackNote}</Typography> : null}

      <FeedbackDialog open={feedbackOpen} onSent={handleFeedbackSent} onClose={() => setFeedbackOpen(false)} />
    </Box>
  );
}
