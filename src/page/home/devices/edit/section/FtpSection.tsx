import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ConnFields } from './shared/ConnFields.tsx';
import { BasicFields } from './shared/BasicFields.tsx';
import type { ConfigTermBase } from '@/common/model/ConfigTermBase.ts';
import { FTP_SECURITY, type ConfigFtp } from '@/common/model/ConfigFtp.ts';
import { SwitchField } from '@/page/common/components/input/SwitchField.tsx';
import { SelectField, type SelectOption } from '@/page/common/components/input/SelectField.tsx';

export function FtpSection(props: { config?: ConfigFtp | null; onChange: (config: ConfigFtp) => void }) {
  const { t } = useI18n();
  const { config, onChange } = props;

  if (!config) {
    return null;
  }

  const base = config.base;
  const securityOptions: SelectOption[] = [
    { value: String(FTP_SECURITY.NONE), label: t('home.devices.ftp.securityNone') },
    { value: String(FTP_SECURITY.EXPLICIT), label: t('home.devices.ftp.securityExplicit') },
    { value: String(FTP_SECURITY.IMPLICIT), label: t('home.devices.ftp.securityImplicit') },
  ];

  const updateBase = (next: ConfigTermBase): void => {
    onChange({ ...config, base: next });
  };

  const handleSecurityChange = (raw: string): void => {
    onChange({ ...config, security: Number(raw) });
  };

  const handleIgnoreCertChange = (ignore_cert: boolean): void => {
    onChange({ ...config, ignore_cert });
  };

  return (
    <Box className="tm-device-edit-fields">
      {base ? <BasicFields connection={base} onChange={updateBase} portPlaceholder={t('home.devices.ftp.portPlaceholder')} usernamePlaceholder={t('home.devices.ftp.usernamePlaceholder')} /> : null}
      <SelectField label={t('home.devices.ftp.security')} value={String(config.security)} onChange={handleSecurityChange} options={securityOptions} />
      {config.security === FTP_SECURITY.NONE ? null : <SwitchField label={t('home.devices.ftp.ignoreCert')} value={config.ignore_cert} onChange={handleIgnoreCertChange} description={t('home.devices.ftp.ignoreCertDesc')} />}
      {base ? <ConnFields connection={base} onChange={updateBase} /> : null}
    </Box>
  );
}
