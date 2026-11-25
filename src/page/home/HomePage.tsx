import { JobPage } from './job/JobPage';
import { SetPage } from './set/SetPage';
import { Sidebar } from './sidebar/Sidebar';
import { DevicePage } from './devices/DevicePage';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ForwardPage } from './forward/ForwardPage';
import { HOME_MENU } from '../common/model/HomeMenu';
import { KeyPage } from '@/page/home/keys/KeyPage.tsx';
import { HAPTIC_KIND } from '@/common/model/dto/Haptic.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { RecordPage } from '@/page/home/record/RecordPage.tsx';
import { SnippetPage } from '@/page/home/snippet/SnippetPage.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { usePagePanelNav } from '@/page/common/hooks/usePagePanelNav.ts';
import { memo, startTransition, useEffect, useRef, useState } from 'react';
import { AgentSplitDivider } from '@/page/agent/common/AgentSplitDivider.tsx';
import { Code, LaptopMac, Settings, VpnKeyOutlined } from '@mui/icons-material';
import { registerGuidanceHomeMenu } from '@/page/common/util/guidanceSurface.ts';

const KeyPageMemo = memo(KeyPage);
const JobPageMemo = memo(JobPage);
const RecordPageMemo = memo(RecordPage);
const DevicePageMemo = memo(DevicePage);
const ForwardPageMemo = memo(ForwardPage);
const SnippetPageMemo = memo(SnippetPage);

const MOBILE_BOTTOM_ITEMS = [
  { menu: HOME_MENU.DEVICES, labelKey: 'home.menu.connections', icon: <LaptopMac className="tm-home-bottom-nav-icon" /> },
  { menu: HOME_MENU.KEYS, labelKey: 'home.menu.keys', icon: <VpnKeyOutlined className="tm-home-bottom-nav-icon" /> },
  { menu: HOME_MENU.SNIPPETS, labelKey: 'home.menu.snippets', icon: <Code className="tm-home-bottom-nav-icon" /> },
  { menu: HOME_MENU.SET, labelKey: 'home.menu.settings', icon: <Settings className="tm-home-bottom-nav-icon" /> },
];

function isMobileHiddenMenu(menu: number): boolean {
  return menu === HOME_MENU.FORWARD || menu === HOME_MENU.JOBS || menu === HOME_MENU.RECORDS;
}

function isHiddenMenu(menu: number, mobileViewport: boolean): boolean {
  return mobileViewport && isMobileHiddenMenu(menu);
}

export function HomePage() {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const rootRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const [leftWidth, setLeftWidth] = useState(220);
  const systemUtilService = useSystemUtilService();
  const [setDialogOpen, setSetDialogOpen] = useState<boolean>(false);
  const [activeMenu, setActiveMenu] = useState<number>(HOME_MENU.DEVICES);
  const [currentMenu, setCurrentMenu] = useState<number>(HOME_MENU.DEVICES);
  const { resolvePanelProps, onPanelAnimationEnd } = usePagePanelNav(currentMenu, false);

  const triggerSelectionHaptic = (): void => {
    if (!mobileViewport) {
      return;
    }
    void systemUtilService.haptic({ kind: HAPTIC_KIND.SELECTION });
  };

  const handleCloseSetDialog = (): void => {
    setSetDialogOpen(false);
  };

  const handleBackToDevices = (): void => {
    setCurrentMenu(HOME_MENU.DEVICES);
    startTransition(() => {
      setActiveMenu(HOME_MENU.DEVICES);
    });
  };

  const handleMenuChange = (menu: number): void => {
    if (isHiddenMenu(menu, mobileViewport)) {
      return;
    }
    triggerSelectionHaptic();
    if (menu === HOME_MENU.SET && !mobileViewport) {
      setSetDialogOpen(true);
      return;
    }
    if (setDialogOpen) {
      setSetDialogOpen(false);
    }
    if (menu === currentMenu) {
      return;
    }
    setCurrentMenu(menu);
    startTransition(() => {
      setActiveMenu(menu);
    });
  };

  useEffect(() => {
    return registerGuidanceHomeMenu((menu: number) => {
      if (isHiddenMenu(menu, mobileViewport)) {
        return;
      }
      setSetDialogOpen(false);
      setCurrentMenu(menu);
      startTransition(() => {
        setActiveMenu(menu);
      });
    });
  }, [mobileViewport]);

  useEffect(() => {
    if (!isHiddenMenu(currentMenu, mobileViewport)) {
      return;
    }
    setCurrentMenu(HOME_MENU.DEVICES);
    startTransition(() => {
      setActiveMenu(HOME_MENU.DEVICES);
    });
  }, [currentMenu, mobileViewport]);

  return (
    <Box ref={rootRef} className="tm-home-root">
      {!mobileViewport ? (
        <>
          <Sidebar rootRef={sidebarRef} width={leftWidth} currentMenu={currentMenu} onMenuChange={handleMenuChange} mobileViewport={false} mobileOpen={false} onMobileClose={() => undefined} />
          <AgentSplitDivider containerRef={rootRef} widthTargetRef={sidebarRef} onWidthChange={setLeftWidth} />
        </>
      ) : null}
      <Box className="tm-home-main">
        <Box {...resolvePanelProps(HOME_MENU.DEVICES)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.DEVICES, event)}>
          <DevicePageMemo active={activeMenu === HOME_MENU.DEVICES} />
        </Box>
        <Box {...resolvePanelProps(HOME_MENU.KEYS)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.KEYS, event)}>
          <KeyPageMemo active={activeMenu === HOME_MENU.KEYS} />
        </Box>
        {!mobileViewport ? (
          <Box {...resolvePanelProps(HOME_MENU.FORWARD)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.FORWARD, event)}>
            <ForwardPageMemo active={activeMenu === HOME_MENU.FORWARD} />
          </Box>
        ) : null}
        <Box {...resolvePanelProps(HOME_MENU.SNIPPETS)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.SNIPPETS, event)}>
          <SnippetPageMemo active={activeMenu === HOME_MENU.SNIPPETS} />
        </Box>
        {!mobileViewport ? (
          <Box {...resolvePanelProps(HOME_MENU.RECORDS)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.RECORDS, event)}>
            <RecordPageMemo active={activeMenu === HOME_MENU.RECORDS} />
          </Box>
        ) : null}
        {!mobileViewport ? (
          <Box {...resolvePanelProps(HOME_MENU.JOBS)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.JOBS, event)}>
            <JobPageMemo active={activeMenu === HOME_MENU.JOBS} />
          </Box>
        ) : null}
        {mobileViewport ? (
          <Box {...resolvePanelProps(HOME_MENU.SET)} onAnimationEnd={event => onPanelAnimationEnd(HOME_MENU.SET, event)}>
            <SetPage open={activeMenu === HOME_MENU.SET} onClose={handleBackToDevices} />
          </Box>
        ) : null}
      </Box>
      {mobileViewport ? (
        <Box className="tm-home-bottom-nav">
          {MOBILE_BOTTOM_ITEMS.map(item => (
            <ButtonBase key={item.menu} disableRipple className={currentMenu === item.menu ? 'tm-home-bottom-nav-item is-selected' : 'tm-home-bottom-nav-item'} data-tm-nav-menu={item.menu} onClick={() => handleMenuChange(item.menu)}>
              <Box component="span" className="tm-home-bottom-nav-indicator" />
              {item.icon}
              <Typography className="tm-home-bottom-nav-label">{t(item.labelKey)}</Typography>
            </ButtonBase>
          ))}
        </Box>
      ) : null}
      {!mobileViewport ? <SetPage open={setDialogOpen} onClose={handleCloseSetDialog} /> : null}
    </Box>
  );
}
