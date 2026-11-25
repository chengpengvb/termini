import Close from '@mui/icons-material/Close';
import { Box, InputBase, Stack } from '@mui/material';
import { useSftpService } from '@/common/context/useAppHooks.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { KeyboardEvent, MouseEvent, useEffect, useRef, useState } from 'react';

interface PathEditProps {
  deviceId: string;
  initialPath: string;
  onCancel: () => void;
  onSubmit: (path: string) => void;
}

export function PathEdit(props: PathEditProps) {
  const sftpService = useSftpService();
  const mobileViewport = usePlatformMobile();
  const rootRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const { deviceId, initialPath, onSubmit, onCancel } = props;
  const [draftPath, setDraftPath] = useState(initialPath);
  const [historyPaths, setHistoryPaths] = useState<string[]>([]);

  const handleSubmit = (): void => {
    const path = draftPath.trim();
    if (!path) {
      return;
    }
    setHistoryPaths(sftpService.addPathHistory(deviceId, path));
    onSubmit(path);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Enter') {
      event.preventDefault();
      handleSubmit();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      onCancel();
    }
  };

  const handleHistoryClick = (path: string): void => {
    setDraftPath(path);
    setHistoryPaths(sftpService.addPathHistory(deviceId, path));
    onSubmit(path);
  };

  const handleRemoveHistory = (event: MouseEvent, path: string): void => {
    event.preventDefault();
    event.stopPropagation();
    setHistoryPaths(sftpService.removePathHistory(deviceId, path));
    inputRef.current?.focus({preventScroll: true});
  };

  const handleClearDraft = (event: MouseEvent): void => {
    event.preventDefault();
    event.stopPropagation();
    setDraftPath('');
    inputRef.current?.focus({preventScroll: true});
  };

  const handleStopMouseDown = (event: MouseEvent): void => {
    event.preventDefault();
  };

  useEffect(() => {
    setHistoryPaths(sftpService.getPathHistory(deviceId));
    inputRef.current?.focus({preventScroll: true});
    inputRef.current?.select();
  }, [deviceId, initialPath, sftpService]);

  useEffect(() => {
    if (!mobileViewport) {
      return;
    }
    const handlePointerDown = (event: PointerEvent): void => {
      const root = rootRef.current;
      if (!root) {
        return;
      }
      const target = event.target;
      if (target instanceof Node && root.contains(target)) {
        return;
      }
      onCancel();
    };
    document.addEventListener('pointerdown', handlePointerDown);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
    };
  }, [mobileViewport, onCancel]);

  return (
    <Box ref={rootRef} className="tm-sftp-path-edit">
      <Stack direction="row" className="tm-sftp-path-edit-input-row">
        <InputBase inputRef={inputRef} value={draftPath} onChange={event => setDraftPath(event.target.value)} onKeyDown={handleKeyDown} onBlur={mobileViewport ? undefined : onCancel} className="tm-sftp-path-edit-input" fullWidth />
        {draftPath ? (
          <Box className="tm-sftp-path-edit-clear" onMouseDown={handleStopMouseDown} onClick={handleClearDraft}>
            <Close className="tm-sftp-path-edit-clear-icon" />
          </Box>
        ) : null}
      </Stack>
      {historyPaths.length > 0 ? (
        <Box className="tm-sftp-path-edit-history" onMouseDown={handleStopMouseDown}>
          {historyPaths.map(path => (
            <Stack key={path} direction="row" className="tm-sftp-path-edit-history-item" onClick={() => handleHistoryClick(path)}>
              <Box className="tm-sftp-path-edit-history-text">{path}</Box>
              <Box className="tm-sftp-path-edit-history-remove" onClick={event => handleRemoveHistory(event, path)}>
                <Close className="tm-sftp-path-edit-history-remove-icon" />
              </Box>
            </Stack>
          ))}
        </Box>
      ) : null}
    </Box>
  );
}
