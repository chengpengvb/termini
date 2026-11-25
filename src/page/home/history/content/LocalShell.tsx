import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { usePaneContext } from '@/common/components/tab';
import { ShellDescr } from '@/common/model/ShellDescr.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { ShellPage } from '@/page/protocol/shell/ShellPage.tsx';
import { RespPlatformInfo } from '@/common/model/resp/RespPlatformInfo.ts';
import { usePlatformNativeMobile } from '@/common/context/usePlatformMobile';
import { resolveShellIcon, resolveShellKind, resolveShellLabel } from '@/page/common/util/shellUtil.ts';
import { useShellService, useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export function LocalShell() {
  const { t } = useI18n();
  const { paneId } = usePaneContext();
  const shellService = useShellService();
  const nativeMobile = usePlatformNativeMobile();
  const systemUtilService = useSystemUtilService();
  const tabManagerService = useTabManagerService();
  const [list, setList] = useState<ShellDescr[]>([]);
  const [platform, setPlatform] = useState<RespPlatformInfo | null>(null);

  const handleLaunch = (item: ShellDescr): void => {
    const kind = resolveShellKind(item.shell_id);
    const icon = resolveShellIcon(kind);
    const label = resolveShellLabel(kind);
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            canClose: true,
            icon,
            title: label,
          },
          content: <ShellPage shellDescr={item} />,
        },
      ],
    });
    tabManagerService.closePane(paneId);
  };

  useEffect(() => {
    if (nativeMobile) {
      return;
    }
    void systemUtilService.getPlatformInfo().then(info => {
      setPlatform(info);
    });
  }, [nativeMobile, systemUtilService]);

  useEffect(() => {
    if (nativeMobile || !platform?.has_local_shell) {
      return;
    }
    void shellService.discover().then(items => {
      setList(items);
    });
  }, [nativeMobile, platform, shellService]);

  if (nativeMobile || !platform?.has_local_shell || list.length === 0) {
    return null;
  }

  return (
    <Box className="tm-history-section">
      <Typography className="tm-history-section-title">{t('home.history.localShell')}</Typography>
      <Box className="tm-history-shell-grid">
        {list.map(item => {
          const kind = resolveShellKind(item.shell_id);
          return (
            <ButtonBase key={item.shell_id} disableRipple className="tm-history-shell-card" onClick={() => handleLaunch(item)}>
              <Box className="tm-history-shell-card-icon" aria-hidden>
                {resolveShellIcon(kind)}
              </Box>
              <Typography className="tm-history-shell-card-label tm-ellipsis">{resolveShellLabel(kind)}</Typography>
            </ButtonBase>
          );
        })}
      </Box>
    </Box>
  );
}
