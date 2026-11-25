import { useEffect, useState } from 'react';
import { ChevronRight } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { About } from '@/page/home/set/about/About.tsx';
import { SET_MENU } from '@/page/common/model/SetMenu.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { Ability } from '@/page/home/set/ability/Ability.tsx';
import { Provider } from '@/page/home/set/agent/Provider.tsx';
import { RecordPage } from '@/page/home/record/RecordPage.tsx';
import { MobileHeader } from '@/page/home/components/MobileHeader.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import type { AboutInfo } from '@/common/model/resp/RespAppVersion.ts';
import { GeneralSection } from '@/page/home/set/system/GeneralSection.tsx';
import { useAccountService, useSetService } from '@/common/context/useAppHooks.tsx';

type NavRow = { menu: number; labelKey: string; hintKey: string };

const DETAIL_MENUS: number[] = [SET_MENU.MODEL, SET_MENU.ABILITY, SET_MENU.RECORDS, SET_MENU.ABOUT];

const AI_ROWS: NavRow[] = [
  { menu: SET_MENU.MODEL, labelKey: 'set.menu.model', hintKey: 'set.mobile.modelHint' },
  { menu: SET_MENU.ABILITY, labelKey: 'set.menu.ability', hintKey: 'set.mobile.abilityHint' },
];

const SESSION_ROWS: NavRow[] = [{ menu: SET_MENU.RECORDS, labelKey: 'home.menu.records', hintKey: 'set.mobile.recordsHint' }];

function resolveDetailMenu(selectedMenu: number): number | null {
  if (DETAIL_MENUS.includes(selectedMenu)) {
    return selectedMenu;
  }
  return null;
}

export function SetMobilePage(props: { open: boolean; onClose: () => void }) {
  const { t } = useI18n();
  const { open, onClose } = props;
  const setService = useSetService();
  const accountService = useAccountService();
  const [drilled, setDrilled] = useState(false);
  const [about, setAbout] = useState<AboutInfo | null>(null);
  const [selectedMenu, setSelectedMenu] = useState<number>(setService.getSelectedMenu());
  const detailMenu = resolveDetailMenu(selectedMenu);

  const resolveRootClass = (): string => {
    if (drilled) {
      return 'tm-set-root is-mobile is-drilled';
    }
    return 'tm-set-root is-mobile';
  };

  const resolveDetailTitle = (): string => {
    if (detailMenu === SET_MENU.RECORDS) {
      return t('home.menu.records');
    }
    if (detailMenu === SET_MENU.MODEL) {
      return t('set.menu.model');
    }
    if (detailMenu === SET_MENU.ABILITY) {
      return t('set.menu.ability');
    }
    return t('set.menu.about');
  };

  const resolveVersionHint = (): string => {
    if (about === null) {
      return t('set.mobile.aboutHint');
    }
    return `${t('set.about.version')} ${about.version}`;
  };

  const handleSelectMenu = (menu: number): void => {
    setSelectedMenu(menu);
    setService.setSelectedMenu(menu);
    setDrilled(true);
  };

  const handleUndrill = (): void => {
    setDrilled(false);
  };

  const renderNavRow = (row: NavRow) => {
    return (
      <ButtonBase key={row.menu} disableRipple className="tm-set-nav-row" onClick={() => handleSelectMenu(row.menu)}>
        <Box className="tm-set-nav-row-main">
          <Typography className="tm-set-nav-row-label">{t(row.labelKey)}</Typography>
          <Typography className="tm-set-nav-row-hint">{t(row.hintKey)}</Typography>
        </Box>
        <ChevronRight className="tm-set-nav-row-chevron" />
      </ButtonBase>
    );
  };

  const renderNavCard = (title: string, rows: NavRow[]) => {
    return (
      <Box className="tm-set-card">
        <Box className="tm-set-card-head">
          <Box className="tm-set-card-head-main">
            <Typography className="tm-set-card-title">{title}</Typography>
          </Box>
        </Box>
        <Box className="tm-set-card-body">{rows.map(renderNavRow)}</Box>
      </Box>
    );
  };

  const renderDetail = () => {
    if (detailMenu === SET_MENU.MODEL) {
      return <Provider />;
    }
    if (detailMenu === SET_MENU.ABILITY) {
      return <Ability />;
    }
    if (detailMenu === SET_MENU.RECORDS) {
      return <RecordPage active={open && drilled} />;
    }
    if (detailMenu === SET_MENU.ABOUT) {
      return <About />;
    }
    return null;
  };

  useBackDismiss(open && drilled, handleUndrill, 'set-page-drill');
  useBackDismiss(open && !drilled, onClose, 'set-page');

  useEffect(() => {
    if (!open) {
      setDrilled(false);
    }
  }, [open]);

  useEffect(() => {
    if (detailMenu !== null) {
      return;
    }
    setDrilled(false);
  }, [detailMenu]);

  useEffect(() => {
    accountService
      .about()
      .then(setAbout)
      .catch(() => {
        setAbout(null);
      });
  }, [accountService]);

  return (
    <Box className={resolveRootClass()}>
      <Box className="tm-set-body">
        <Box className="tm-set-page is-menu">
          <MobileHeader title={t('set.title')} />
          <Box className="tm-set-mobile-scroll">
            <GeneralSection />
            {renderNavCard(t('set.group.ai'), AI_ROWS)}
            {renderNavCard(t('set.group.session'), SESSION_ROWS)}
            <Box className="tm-set-card">
              <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                  <Typography className="tm-set-card-title">{t('set.menu.about')}</Typography>
                </Box>
              </Box>
              <Box className="tm-set-card-body">
                <ButtonBase disableRipple className="tm-set-nav-row" onClick={() => handleSelectMenu(SET_MENU.ABOUT)}>
                  <Box className="tm-set-nav-row-main">
                    <Typography className="tm-set-nav-row-label">Termini</Typography>
                    <Typography className="tm-set-nav-row-hint">{resolveVersionHint()}</Typography>
                  </Box>
                  <ChevronRight className="tm-set-nav-row-chevron" />
                </ButtonBase>
              </Box>
            </Box>
          </Box>
        </Box>
        <Box className="tm-set-page is-detail">
          <MobileHeader title={resolveDetailTitle()} onBack={handleUndrill} />
          <Box className="tm-set-mobile-detail">{renderDetail()}</Box>
        </Box>
      </Box>
    </Box>
  );
}
