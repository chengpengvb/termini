import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotNil } from '@/common/util/StringUtil.ts';
import { Box, Button, Switch, Typography } from '@mui/material';
import { TextareaField } from '@/page/common/components/input/TextareaField.tsx';
import { useCodeIndexService, useConfigService } from '@/common/context/useAppHooks.tsx';
import { INDEX_DIR_STATE, type IndexDirStatus } from '@/common/model/resp/RespCodeIndex.ts';

const MAX_INDEX_DIRS = 5;
const STATUS_POLL_MS = 5000;

export function CodeIndexSection() {
  const { t, tf } = useI18n();
  const configService = useConfigService();
  const codeIndexService = useCodeIndexService();
  const [ignore, setIgnore] = useState<string>('');
  const [busy, setBusy] = useState<boolean>(false);
  const [dirs, setDirs] = useState<IndexDirStatus[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [includeTests, setIncludeTests] = useState<boolean>(false);

  const refreshDirs = (): void => {
    codeIndexService
      .status()
      .then(setDirs)
      .catch(() => setDirs([]));
  };

  const applyDirs = (next: string[]): void => {
    setBusy(true);
    setError(null);
    codeIndexService
      .saveDirs(next)
      .then(rejected => {
        if (rejected.length > 0) {
          setError(rejected[0]);
        }
        refreshDirs();
      })
      .catch(() => setError(t('common.saveFailed')))
      .finally(() => setBusy(false));
  };

  const handleAddDir = (): void => {
    if (dirs.length >= MAX_INDEX_DIRS) {
      setError(tf('set.codeIndex.dirLimit', { n: MAX_INDEX_DIRS }));
      return;
    }
    void open({ directory: true, multiple: false }).then(selected => {
      if (selected === null) {
        return;
      }
      applyDirs([...dirs.map(item => item.dir), selected]);
    });
  };

  const handleRemoveDir = (dir: string): void => {
    applyDirs(dirs.filter(item => item.dir !== dir).map(item => item.dir));
  };

  const handleRebuild = (dir: string): void => {
    setError(null);
    codeIndexService
      .rebuild(dir)
      .then(() => refreshDirs())
      .catch(() => setError(t('common.saveFailed')));
  };

  const resolveStateText = (row: IndexDirStatus): string => {
    if (row.state === INDEX_DIR_STATE.BUILDING) {
      return isNotNil(row.stage) ? tf('set.codeIndex.stateBuildingStage', { stage: row.stage }) : t('set.codeIndex.stateBuilding');
    }
    if (row.state === INDEX_DIR_STATE.PENDING) {
      return t('set.codeIndex.statePending');
    }
    if (row.state === INDEX_DIR_STATE.FAILED) {
      return isNotNil(row.error) ? row.error : t('set.codeIndex.stateFailed');
    }
    if (row.state === INDEX_DIR_STATE.READY || row.chunkCount > 0) {
      return tf('set.codeIndex.stateReady', { n: row.chunkCount });
    }
    return t('set.codeIndex.stateIdle');
  };

  const persist = (nextIgnore: string, nextIncludeTests: boolean): void => {
    setError(null);
    const value = nextIgnore.trim().length === 0 ? null : nextIgnore;
    configService.saveCodeIndex(value, nextIncludeTests).catch(() => {
      setError(t('common.saveFailed'));
    });
  };

  const handleIncludeTestsChange = (next: boolean): void => {
    setIncludeTests(next);
    persist(ignore, next);
  };

  const renderError = (): ReactNode => {
    if (!isNotNil(error)) {
      return null;
    }
    return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
  };

  const renderDirs = (): ReactNode => {
    if (dirs.length === 0) {
      return (
        <Box className="tm-set-row">
          <Typography className="tm-set-row-hint">{t('set.codeIndex.dirsEmpty')}</Typography>
        </Box>
      );
    }
    return dirs.map(row => (
      <Box key={row.dir} className="tm-set-row">
        <Box className="tm-set-row-main">
          <Typography className="tm-set-index-dir-path" title={row.dir}>
            {row.dir}
          </Typography>
          <Typography className={row.state === INDEX_DIR_STATE.FAILED ? 'tm-set-index-dir-state is-failed' : 'tm-set-index-dir-state'}>{resolveStateText(row)}</Typography>
        </Box>
        <Box className="tm-set-index-dir-actions">
          <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" disabled={busy || row.state === INDEX_DIR_STATE.BUILDING} onClick={() => handleRebuild(row.dir)}>
            {t('set.codeIndex.rebuild')}
          </Button>
          <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" disabled={busy} onClick={() => handleRemoveDir(row.dir)}>
            {t('set.codeIndex.removeDir')}
          </Button>
        </Box>
      </Box>
    ));
  };

  useEffect(() => {
    configService
      .selectConfig()
      .then(config => {
        setIgnore(config?.index_ignore ?? '');
        setIncludeTests(Boolean(config?.index_include_tests));
      })
      .catch(() => {
        setIgnore('');
      });
  }, [configService]);

  // 建索引在后台跑, 轮询让状态与进度跟着走
  useEffect(() => {
    refreshDirs();
    const timer = window.setInterval(refreshDirs, STATUS_POLL_MS);
    return () => window.clearInterval(timer);
  }, [codeIndexService]);

  return (
    <>
      <Box className="tm-set-card">
        <Box className="tm-set-card-head">
          <Box className="tm-set-card-head-main">
            <Typography className="tm-set-card-title">{t('set.codeIndex.dirs')}</Typography>
            <Typography className="tm-set-card-desc">{tf('set.codeIndex.desc', { n: MAX_INDEX_DIRS })}</Typography>
          </Box>
          <Box className="tm-set-card-head-action">
            {renderError()}
            <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" disabled={busy || dirs.length >= MAX_INDEX_DIRS} onClick={handleAddDir}>
              {t('set.codeIndex.addDir')}
            </Button>
          </Box>
        </Box>
        <Box className="tm-set-card-body">
          {renderDirs()}
          <Box className="tm-set-row">
            <Box className="tm-set-row-main">
              <Typography className="tm-set-row-label">{t('set.codeIndex.includeTests')}</Typography>
              <Typography className="tm-set-row-hint">{t('set.codeIndex.includeTestsHint')}</Typography>
            </Box>
            <Box className="tm-set-row-control">
              <Switch size="small" className="tm-set-switch" checked={includeTests} onChange={e => handleIncludeTestsChange(e.target.checked)} />
            </Box>
          </Box>
        </Box>
      </Box>
      <Box className="tm-set-card">
        <Box className="tm-set-card-head">
          <Box className="tm-set-card-head-main">
            <Typography className="tm-set-card-title">{t('set.codeIndex.ignore')}</Typography>
            <Typography className="tm-set-card-desc">{t('set.codeIndex.ignoreHint')}</Typography>
          </Box>
        </Box>
        <Box className="tm-set-card-body">
          <Box className="tm-set-row is-column">
            <TextareaField value={ignore} onChange={setIgnore} onBlur={() => persist(ignore, includeTests)} placeholder={t('set.codeIndex.ignorePlaceholder')} expandable language="ini" minRows={1} maxRows={1} />
          </Box>
        </Box>
      </Box>
    </>
  );
}
