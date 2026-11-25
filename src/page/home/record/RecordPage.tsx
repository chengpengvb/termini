import { Box } from '@mui/material';
import { TopBar } from './toolbar/TopBar';
import { Content } from './content/Content';
import { PlayerDialog } from './player/PlayerDialog';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { SessionRecord } from '@/common/model/SessionRecord.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useSessionRecordService } from '@/common/context/useAppHooks.tsx';

export function RecordPage(props: { active: boolean }) {
  const { active } = props;
  const [loaded, setLoaded] = useState<boolean>(false);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const sessionRecordService = useSessionRecordService();
  const [error, setError] = useState<string | null>(null);
  const [records, setRecords] = useState<SessionRecord[]>([]);
  const [playerOpen, setPlayerOpen] = useState<boolean>(false);
  const [record, setRecord] = useState<SessionRecord | null>(null);
  const [searchText, setSearchText] = useState<string | null>(null);

  const reload = useCallback((): void => {
    sessionRecordService
      .list()
      .then(list => {
        setRecords(list);
        setError(null);
        setLoaded(true);
      })
      .catch(err => {
        setLoaded(true);
        setError(resolveErrorText(err));
      });
  }, [sessionRecordService]);

  const handlePlay = (value: SessionRecord): void => {
    setRecord(value);
    setPlayerOpen(true);
  };

  const handleClosePlayer = (): void => {
    setPlayerOpen(false);
  };

  useEffect(() => {
    if (!active) {
      return;
    }
    reload();
  }, [active, reload]);

  return (
    <Box className="tm-home-page tm-home-page-column">
      <Box className="tm-agent-page tm-page-column">
        <TopBar searchText={searchText} onSearchChange={setSearchText} onCleaned={reload} />
        <Content loading={!loaded} error={error} records={records} searchText={searchText} scrollRef={scrollRef} onPlay={handlePlay} onDeleted={reload} />
      </Box>
      <PlayerDialog open={playerOpen} record={record} onClose={handleClosePlayer} />
    </Box>
  );
}
