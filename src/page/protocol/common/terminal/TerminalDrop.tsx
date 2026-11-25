import type { RefObject } from 'react';
import { useEffect, useRef } from 'react';
import { inTauri } from '@/common/util/TauriUtil.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { formatTerminalDropPaths } from '@/page/common/util/terminal.ts';
import { getCurrentWebview, type DragDropEvent } from '@tauri-apps/api/webview';
import { isPointInsideElement, physicalPositionToCssPoint, resolveWebviewDragDevicePixelRatio } from '@/page/common/util/sftpUtil.ts';

export interface TerminalDropProps {
  getCwd: () => string | null;
  onInput: (data: string) => void;
  hostRef: RefObject<HTMLDivElement | null>;
}

export function TerminalDrop(props: TerminalDropProps) {
  const { hostRef, getCwd, onInput } = props;
  const getCwdRef = useRef(getCwd);
  const onInputRef = useRef(onInput);
  getCwdRef.current = getCwd;
  onInputRef.current = onInput;

  const handleOsDragDrop = (payload: DragDropEvent): void => {
    if (payload.type !== 'drop') {
      return;
    }
    const host = hostRef.current;
    if (host == null) {
      return;
    }
    const point = physicalPositionToCssPoint(payload.position, resolveWebviewDragDevicePixelRatio());
    if (!isPointInsideElement(host, point.x, point.y)) {
      return;
    }
    const text = formatTerminalDropPaths(payload.paths, getCwdRef.current());
    if (isEmpty(text)) {
      return;
    }
    onInputRef.current(text);
  };

  useEffect(() => {
    if (!inTauri()) {
      return;
    }
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent(event => {
        handleOsDragDrop(event.payload);
      })
      .then(off => {
        if (disposed) {
          off();
          return;
        }
        unlisten = off;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [hostRef]);

  return null;
}
