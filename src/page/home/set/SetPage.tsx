import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { Job } from '@/page/home/set/job/Job.tsx';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { About } from '@/page/home/set/about/About.tsx';
import { SET_MENU } from '@/page/common/model/SetMenu.ts';
import { System } from '@/page/home/set/system/System.tsx';
import { Ability } from '@/page/home/set/ability/Ability.tsx';
import { Provider } from '@/page/home/set/agent/Provider.tsx';
import { CodeIndex } from '@/page/home/set/code/CodeIndex.tsx';
import { useSetService } from '@/common/context/useAppHooks.tsx';
import { SetMobilePage } from '@/page/home/set/SetMobilePage.tsx';
import { Box, ButtonBase, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { AutoAwesome, InfoOutlined, ManageSearch, MonitorHeart, Psychology, Tune } from '@mui/icons-material';

type SetMenuItem = { menu: number; labelKey: string; icon: ReactNode };

const MENU_ITEMS: SetMenuItem[] = [
  { menu: SET_MENU.SYSTEM, labelKey: 'set.menu.system', icon: <Tune className="tm-set-sidebar-icon" /> },
  { menu: SET_MENU.MODEL, labelKey: 'set.menu.model', icon: <Psychology className="tm-set-sidebar-icon" /> },
  { menu: SET_MENU.ABILITY, labelKey: 'set.menu.ability', icon: <AutoAwesome className="tm-set-sidebar-icon" /> },
  { menu: SET_MENU.JOB, labelKey: 'set.menu.job', icon: <MonitorHeart className="tm-set-sidebar-icon" /> },
  { menu: SET_MENU.CODE_INDEX, labelKey: 'set.menu.codeIndex', icon: <ManageSearch className="tm-set-sidebar-icon" /> },
  { menu: SET_MENU.ABOUT, labelKey: 'set.menu.about', icon: <InfoOutlined className="tm-set-sidebar-icon" /> },
];

function resolveDetailMenu(selectedMenu: number): number {
  if (MENU_ITEMS.some(item => item.menu === selectedMenu)) {
    return selectedMenu;
  }
  return SET_MENU.SYSTEM;
}

export function SetPage(props: { open: boolean; onClose: () => void }) {
  const { open, onClose } = props;
  const { t } = useI18n();
  const setService = useSetService();
  const mobileViewport = usePlatformMobile();
  const [selectedMenu, setSelectedMenu] = useState<number>(setService.getSelectedMenu());
  const detailMenu = resolveDetailMenu(selectedMenu);

  const resolveSidebarItemClass = (menu: number): string => {
    if (detailMenu === menu) {
      return 'tm-set-sidebar-item is-selected';
    }
    return 'tm-set-sidebar-item';
  };

  const handleSelectMenu = (menu: number): void => {
    setSelectedMenu(menu);
    setService.setSelectedMenu(menu);
  };

  useEffect(() => {
    if (mobileViewport || selectedMenu === detailMenu) {
      return;
    }
    setSelectedMenu(detailMenu);
    setService.setSelectedMenu(detailMenu);
  }, [detailMenu, selectedMenu, setService, mobileViewport]);

  if (mobileViewport) {
    return <SetMobilePage open={open} onClose={onClose} />;
  }

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-set-dialog-paper' })}>
      <Box className="tm-set-root">
        <Header title={t('set.title')} onClose={onClose} />
        <Box className="tm-set-body">
          <Box className="tm-set-sidebar">
            {MENU_ITEMS.map(item => (
              <ButtonBase key={item.menu} disableRipple className={resolveSidebarItemClass(item.menu)} onClick={() => handleSelectMenu(item.menu)}>
                {item.icon}
                <Typography component="span" className="tm-set-sidebar-label">
                  {t(item.labelKey)}
                </Typography>
              </ButtonBase>
            ))}
          </Box>
          {detailMenu === SET_MENU.MODEL ? <Provider /> : null}
          {detailMenu === SET_MENU.ABILITY ? <Ability /> : null}
          {detailMenu === SET_MENU.JOB ? <Job /> : null}
          {detailMenu === SET_MENU.CODE_INDEX ? <CodeIndex /> : null}
          {detailMenu === SET_MENU.SYSTEM ? <System /> : null}
          {detailMenu === SET_MENU.ABOUT ? <About /> : null}
        </Box>
      </Box>
    </Dialog>
  );
}
