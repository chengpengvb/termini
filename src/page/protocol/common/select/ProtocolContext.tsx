import { useState } from 'react';
import { Box } from '@mui/material';
import { PROTOCOL } from '@/common/const/Const.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { Bottom } from '@/page/protocol/common/select/context/Bottom.tsx';
import { useHistoryLoginService } from '@/common/context/useAppHooks.tsx';
import { resolveSelectableProtocols } from '@/page/common/util/protocolOpenUtil.ts';
import { ProtocolCard } from '@/page/protocol/common/select/context/ProtocolCard.tsx';

export interface ProtocolContextProps {
  reqDevice: ReqDevice;
  doConnect: (protocols: number[]) => void;
}

export function ProtocolContext(props: ProtocolContextProps) {
  const { reqDevice, doConnect } = props;
  const historyLoginService = useHistoryLoginService();
  const lastProtocols = historyLoginService.getLastProtocols(reqDevice.device.id);
  const selectable = resolveSelectableProtocols(reqDevice);
  const initialSelected = lastProtocols.length > 0 ? lastProtocols : selectable.includes(PROTOCOL.SSH) ? [PROTOCOL.SSH] : selectable.slice(0, 1);
  const [selected, setSelected] = useState<number[]>(initialSelected);

  const handleToggle = (protocol: number): void => {
    setSelected(prev => {
      if (prev.includes(protocol)) {
        return prev.filter(p => p !== protocol);
      }
      return [...prev, protocol];
    });
  };

  return (
    <Box sx={{ width: '100%', height: '100%' }}>
      <Box className="tm-protocol-select">
        <ProtocolCard reqDevice={reqDevice} selectedProtocols={selected} onToggle={handleToggle} doConnect={doConnect} />
        <Bottom doConnect={doConnect} selectedProtocols={selected} />
      </Box>
    </Box>
  );
}
