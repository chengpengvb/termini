import { Item } from './Item';
import { Box, Typography } from '@mui/material';
import { DataObject } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import type { Snippet } from '@/common/model/Snippet.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';

export interface ContentProps {
  loading: boolean;
  onAdd: () => void;
  snippets: Snippet[];
  error: string | null;
  onDeleted: () => void;
  searchText: string | null;
  onEdit: (value: Snippet) => void;
}

function matchSnippet(snippet: Snippet, keyword: string): boolean {
  if (snippet.name.toLowerCase().includes(keyword)) {
    return true;
  }
  if (snippet.command.toLowerCase().includes(keyword)) {
    return true;
  }
  return snippet.remark.toLowerCase().includes(keyword);
}

export function Content(props: ContentProps) {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const { loading, error, snippets, searchText, onAdd, onEdit, onDeleted } = props;
  const visible = isEmpty(searchText) ? snippets : snippets.filter(item => matchSnippet(item, searchText.toLowerCase()));

  if (loading && snippets.length === 0) {
    return <Typography className="tm-home-loading-text">{t('home.common.loading')}</Typography>;
  }
  if (error) {
    return <Typography className="tm-home-error-text">{tf('home.common.loadFailed', { error })}</Typography>;
  }
  if (snippets.length === 0) {
    if (!mobileViewport) {
      return (
        <Box className="tm-agent-page-empty" role="button" tabIndex={0} onClick={onAdd}>
          <Typography className="tm-agent-page-empty-title">{t('home.snippet.emptyTitle')}</Typography>
          <Typography className="tm-agent-page-empty-message">{t('home.snippet.emptyMessage')}</Typography>
        </Box>
      );
    }
    return (
      <Box className="tm-empty-state tm-mobile-empty">
        <Box className="tm-empty-state-card">
          <Box className="tm-empty-state-icon">
            <DataObject className="tm-empty-state-icon-svg" />
          </Box>
          <Typography className="tm-empty-state-title" onClick={onAdd}>
            {t('home.snippet.emptyTitle')}
          </Typography>
          <Typography className="tm-empty-state-message">{t('home.snippet.emptyMessage')}</Typography>
          <MobileHeaderAction className="tm-empty-state-cta" label={t('common.add')} onClick={onAdd} />
        </Box>
      </Box>
    );
  }
  if (visible.length === 0) {
    return (
      <Box className="tm-agent-page-empty">
        <Typography className="tm-agent-page-empty-message">{t('home.snippet.noMatch')}</Typography>
      </Box>
    );
  }

  return (
    <Box className="tm-agent-list">
      {visible.map(item => (
        <Item key={item.id} snippet={item} onEdit={onEdit} onDeleted={onDeleted} />
      ))}
    </Box>
  );
}
