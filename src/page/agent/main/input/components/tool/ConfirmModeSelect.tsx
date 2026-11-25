import { useI18n } from '@/common/i18n/useI18n.ts';
import { useRef, useState, type ReactNode } from 'react';
import { Box, ButtonBase, Popover, Typography } from '@mui/material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { OverlayPopoverPaper } from '@/page/common/util/scopedDialogUtil.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { Check, FrontHandOutlined, ShieldOutlined, SmartToyOutlined } from '@mui/icons-material';
import { CONFIRM_MODE_AUTO, CONFIRM_MODE_MANUAL, CONFIRM_MODE_SILENT } from '@/common/model/dto/ConfirmMode.ts';

interface ConfirmModeOption {
  mode: number;
  labelKey: string;
  detailKey: string;
  icon: ReactNode;
  danger?: boolean;
}

const CONFIRM_MODE_OPTIONS: ConfirmModeOption[] = [
  {
    mode: CONFIRM_MODE_MANUAL,
    labelKey: 'agent.confirmMode.manualLabel',
    detailKey: 'agent.confirmMode.manualDetail',
    icon: <FrontHandOutlined />,
  },
  {
    mode: CONFIRM_MODE_AUTO,
    labelKey: 'agent.confirmMode.autoLabel',
    detailKey: 'agent.confirmMode.autoDetail',
    icon: <ShieldOutlined />,
  },
  {
    danger: true,
    mode: CONFIRM_MODE_SILENT,
    labelKey: 'agent.confirmMode.silentLabel',
    detailKey: 'agent.confirmMode.silentDetail',
    icon: <SmartToyOutlined />,
  },
];

interface ConfirmModeSelectProps {
  mode: number;
  onSelect: (mode: number) => void;
}

export function ConfirmModeSelect(props: ConfirmModeSelectProps) {
  const { mode, onSelect } = props;
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const [open, setOpen] = useState<boolean>(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);

  function resolveSelected(): ConfirmModeOption {
    const found = CONFIRM_MODE_OPTIONS.find(item => item.mode === mode);
    if (found) {
      return found;
    }
    return CONFIRM_MODE_OPTIONS[0];
  }

  function handleOpen(): void {
    setOpen(true);
  }

  function handleClose(): void {
    setOpen(false);
  }

  function handlePick(option: ConfirmModeOption): void {
    onSelect(option.mode);
    handleClose();
  }

  function resolveTriggerClass(option: ConfirmModeOption): string {
    const names = ['tm-agent-confirm-trigger'];
    if (option.danger) {
      names.push('is-danger');
    }
    if (open) {
      names.push('is-open');
    }
    return names.join(' ');
  }

  function resolveOptionClass(option: ConfirmModeOption, isSelected: boolean): string {
    const names = ['tm-agent-confirm-option'];
    if (option.danger) {
      names.push('is-danger');
    }
    if (isSelected) {
      names.push('is-selected');
    }
    return names.join(' ');
  }

  const selected = resolveSelected();

  const menuBody = (
    <Box className={mobileViewport ? 'tm-agent-confirm-menu is-sheet' : 'tm-agent-confirm-menu'}>
      <Box className="tm-agent-confirm-header">
        <Typography className="tm-agent-confirm-header-title">{t('agent.confirmMode.header')}</Typography>
      </Box>
      <Box className="tm-agent-confirm-option-list">
        {CONFIRM_MODE_OPTIONS.map(option => {
          const isSelected = option.mode === mode;
          return (
            <ButtonBase key={option.mode} disableRipple onClick={() => handlePick(option)} className={resolveOptionClass(option, isSelected)}>
              <Box component="span" className="tm-agent-confirm-option-icon" aria-hidden>
                {option.icon}
              </Box>
              <Box className="tm-agent-confirm-option-body">
                <Typography className="tm-agent-confirm-option-name">{t(option.labelKey)}</Typography>
                <Typography className="tm-agent-confirm-option-detail">{t(option.detailKey)}</Typography>
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
      <ButtonBase ref={triggerRef} disableRipple onClick={handleOpen} aria-label={t(selected.labelKey)} title={t(selected.labelKey)} className={resolveTriggerClass(selected)}>
        <Box component="span" className="tm-agent-confirm-trigger-icon" aria-hidden>
          {selected.icon}
        </Box>
        <Typography component="span" className="tm-agent-confirm-trigger-label">
          {t(selected.labelKey)}
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
