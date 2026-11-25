import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Button, Dialog } from '@mui/material';
import type { ConfigSerial } from '@/common/model/ConfigSerial.ts';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { SelectField } from '@/page/common/components/input/SelectField.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

const STOP_BITS = [1, 2];
const DATA_BITS = [8, 7, 6, 5];
const BAUD_RATES = [4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];

export interface SerialConnectValue {
  baud_rate: number;
  data_bits: number;
  stop_bits: number;
  parity: string;
  flow_control: string;
}

export function SerialConnectDialog(props: { open: boolean; port: string; config: ConfigSerial | null; onCancel: () => void; onConnect: (value: SerialConnectValue) => void }) {
  const { t } = useI18n();
  const { open, port, config, onCancel, onConnect } = props;
  const [parity, setParity] = useState<string>('none');
  const [baudRate, setBaudRate] = useState<number>(9600);
  const [dataBits, setDataBits] = useState<number>(8);
  const [stopBits, setStopBits] = useState<number>(1);
  const [flowControl, setFlowControl] = useState<string>('none');

  const PARITY_OPTIONS = [
    { value: 'none', label: t('home.devices.serialParityNone') },
    { value: 'odd', label: t('home.devices.serialParityOdd') },
    { value: 'even', label: t('home.devices.serialParityEven') },
  ];

  const FLOW_OPTIONS = [
    { value: 'none', label: t('home.devices.serialFlowNone') },
    { value: 'software', label: t('home.devices.serialFlowSoftware') },
    { value: 'hardware', label: t('home.devices.serialFlowHardware') },
  ];

  const handleConnect = (): void => {
    onConnect({ baud_rate: baudRate, data_bits: dataBits, stop_bits: stopBits, parity, flow_control: flowControl });
  };

  useBackDismiss(open, onCancel, 'serial-connect');

  useEffect(() => {
    if (!open) {
      return;
    }
    setParity(config?.parity ?? 'none');
    setBaudRate(config?.baud_rate ?? 9600);
    setDataBits(config?.data_bits ?? 8);
    setStopBits(config?.stop_bits ?? 1);
    setFlowControl(config?.flow_control ?? 'none');
  }, [open, config]);

  return (
    <Dialog open={open} onClose={onCancel} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-serial-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={`${t('home.devices.serialDialogTitle')} · ${port}`} onClose={onCancel} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <SelectField label={t('home.devices.serialBaudRate')} value={String(baudRate)} onChange={v => setBaudRate(Number(v))} options={BAUD_RATES.map(rate => ({ value: String(rate), label: String(rate) }))} />
          <SelectField label={t('home.devices.serialDataBits')} value={String(dataBits)} onChange={v => setDataBits(Number(v))} options={DATA_BITS.map(bits => ({ value: String(bits), label: String(bits) }))} />
          <SelectField label={t('home.devices.serialStopBits')} value={String(stopBits)} onChange={v => setStopBits(Number(v))} options={STOP_BITS.map(bits => ({ value: String(bits), label: String(bits) }))} />
          <SelectField label={t('home.devices.serialParity')} value={parity} onChange={setParity} options={PARITY_OPTIONS} />
          <SelectField label={t('home.devices.serialFlowControl')} value={flowControl} onChange={setFlowControl} options={FLOW_OPTIONS} />
        </Box>
        <Box className="tm-edit-dialog-footer">
          <Button onClick={onCancel} className="tm-btn tm-btn-secondary">
            {t('common.cancel')}
          </Button>
          <Box className="tm-edit-dialog-footer-spacer" />
          <Button onClick={handleConnect} className="tm-btn tm-btn-primary">
            {t('home.common.connect')}
          </Button>
        </Box>
      </Box>
    </Dialog>
  );
}
