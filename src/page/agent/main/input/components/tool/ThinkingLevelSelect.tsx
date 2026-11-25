import { useI18n } from '@/common/i18n/useI18n.ts';
import { useRef, useState, type ReactNode } from 'react';
import { Box, ButtonBase, Popover, Typography } from '@mui/material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { OverlayPopoverPaper } from '@/page/common/util/scopedDialogUtil.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { AutoAwesomeOutlined, BoltOutlined, Check, PsychologyOutlined } from '@mui/icons-material';

const KNOWN_LEVELS: Record<string, { labelKey: string; detailKey: string; icon: ReactNode }> = {
  low: { labelKey: 'agent.thinkingLevel.low', detailKey: 'agent.thinkingLevel.lowDetail', icon: <BoltOutlined /> },
  high: { labelKey: 'agent.thinkingLevel.high', detailKey: 'agent.thinkingLevel.highDetail', icon: <PsychologyOutlined /> },
  max: { labelKey: 'agent.thinkingLevel.max', detailKey: 'agent.thinkingLevel.maxDetail', icon: <AutoAwesomeOutlined /> },
};

interface ThinkingLevelSelectProps {
  levels: string[];
  level: string | null;
  onSelect: (level: string) => void;
}

export function ThinkingLevelSelect(props: ThinkingLevelSelectProps) {
  const { t } = useI18n();
  const { levels, level, onSelect } = props;
  const mobileViewport = usePlatformMobile();
  const [open, setOpen] = useState<boolean>(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const current = level !== null && levels.includes(level) ? level : levels[0];

  function resolveLabel(name: string): string {
    const known = KNOWN_LEVELS[name];
    return known ? t(known.labelKey) : name;
  }

  function resolveDetail(name: string): string | null {
    const known = KNOWN_LEVELS[name];
    return known ? t(known.detailKey) : null;
  }

  function resolveIcon(name: string): ReactNode {
    const known = KNOWN_LEVELS[name];
    return known ? known.icon : <PsychologyOutlined />;
  }

  function handleOpen(): void {
    setOpen(true);
  }

  function handleClose(): void {
    setOpen(false);
  }

  function handlePick(name: string): void {
    onSelect(name);
    handleClose();
  }

  if (levels.length === 0) {
    return null;
  }

  const menuBody = (
    <Box className={mobileViewport ? 'tm-agent-confirm-menu is-sheet' : 'tm-agent-confirm-menu'}>
      <Box className="tm-agent-confirm-header">
        <Typography className="tm-agent-confirm-header-title">{t('agent.thinkingLevel.header')}</Typography>
      </Box>
      <Box className="tm-agent-confirm-option-list">
        {levels.map(name => {
          const isSelected = name === current;
          const detail = resolveDetail(name);
          return (
            <ButtonBase key={name} disableRipple onClick={() => handlePick(name)} className={isSelected ? 'tm-agent-confirm-option is-selected' : 'tm-agent-confirm-option'}>
              <Box component="span" className="tm-agent-confirm-option-icon" aria-hidden>
                {resolveIcon(name)}
              </Box>
              <Box className="tm-agent-confirm-option-body">
                <Typography className="tm-agent-confirm-option-name">{resolveLabel(name)}</Typography>
                {detail !== null ? <Typography className="tm-agent-confirm-option-detail">{detail}</Typography> : null}
              </Box>
              <Box className="tm-agent-confirm-option-check">{isSelected ? <Check sx={{ fontSize: 16 }} /> : null}</Box>
            </ButtonBase>
          );
        })}
      </Box>
    </Box>
  );

  return (
    <>
      <ButtonBase ref={triggerRef} disableRipple onClick={handleOpen} aria-label={t('agent.thinkingLevel.header')} className={open ? 'tm-agent-confirm-trigger is-open' : 'tm-agent-confirm-trigger'} title={t('agent.thinkingLevel.header')}>
        <Box component="span" className="tm-agent-confirm-trigger-icon" aria-hidden>
          {resolveIcon(current)}
        </Box>
        <Typography component="span" className="tm-agent-confirm-trigger-label">
          {resolveLabel(current)}
        </Typography>
      </ButtonBase>
      {mobileViewport ? (
        <ActionSheet open={open} onClose={handleClose}>
          {menuBody}
        </ActionSheet>
      ) : (
        <Popover
          open={open}
          anchorEl={triggerRef.current}
          onClose={handleClose}
          anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
          transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
          slots={{ paper: OverlayPopoverPaper }}
          slotProps={{ paper: { className: 'tm-agent-confirm-menu-paper' } }}
        >
          {menuBody}
        </Popover>
      )}
    </>
  );
}
