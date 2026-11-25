import { Item } from './Item';
import type { RefObject } from 'react';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { SessionRecord } from '@/common/model/SessionRecord.ts';

export interface ContentProps {
  loading: boolean;
  error: string | null;
  onDeleted: () => void;
  records: SessionRecord[];
  searchText: string | null;
  onPlay: (value: SessionRecord) => void;
  scrollRef: RefObject<HTMLDivElement | null>;
}

const ROW_HEIGHT = 56;
const ROW_GAP = 2;

function matchRecord(record: SessionRecord, keyword: string): boolean {
  return record.device_name.toLowerCase().includes(keyword) || record.protocol.toLowerCase().includes(keyword);
}

export function Content(props: ContentProps) {
  const { t, tf } = useI18n();
  const { loading, error, records, searchText, scrollRef, onPlay, onDeleted } = props;
  const visible = isEmpty(searchText) ? records : records.filter(item => matchRecord(item, searchText.toLowerCase()));

  const rowVirtualizer = useVirtualizer({
    gap: ROW_GAP,
    overscan: 8,
    count: visible.length,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: index => visible[index].id,
    getScrollElement: () => scrollRef.current,
  });

  if (loading && records.length === 0) {
    return <Typography className="tm-home-loading-text">{t('home.common.loading')}</Typography>;
  }
  if (error) {
    return <Typography className="tm-home-error-text">{tf('home.common.loadFailed', { error })}</Typography>;
  }
  if (records.length === 0) {
    return (
      <Box className="tm-agent-page-empty">
        <Typography className="tm-agent-page-empty-title">{t('home.record.emptyTitle')}</Typography>
        <Typography className="tm-agent-page-empty-message">{t('home.record.emptyMessage')}</Typography>
      </Box>
    );
  }
  if (visible.length === 0) {
    return (
      <Box className="tm-agent-page-empty">
        <Typography className="tm-agent-page-empty-message">{t('home.record.noMatch')}</Typography>
      </Box>
    );
  }

  return (
    <Box ref={scrollRef} className="tm-agent-list is-virtual">
      <Box style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
        {rowVirtualizer.getVirtualItems().map(virtualRow => {
          const record = visible[virtualRow.index];
          return (
            <Box key={record.id} data-index={virtualRow.index} ref={rowVirtualizer.measureElement} className="tm-agent-list-virtual-row" style={{ transform: `translateY(${virtualRow.start}px)` }}>
              <Item record={record} onPlay={onPlay} onDeleted={onDeleted} />
            </Box>
          );
        })}
      </Box>
    </Box>
  );
}
