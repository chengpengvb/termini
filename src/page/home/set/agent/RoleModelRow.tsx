import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { ModelRole } from '@/common/model/Config.ts';
import { isEmpty, isNotEmpty } from '@/common/util/StringUtil.ts';
import { useConfigService } from '@/common/context/useAppHooks.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { resolveAiProvideModelLabel } from '@/common/model/AiProvide.ts';
import { useAiProvideOptions } from '@/page/common/hooks/useAiProvideOptions.ts';
import { SelectField, type SelectOption } from '@/page/common/components/input/SelectField.tsx';

const USE_DEFAULT = '';

export interface RoleModelRowProps {
  role: ModelRole;
  label: string;
  hint: string;
  useCloud: boolean;
  provideId: string;
  dim: number | null;
  withDim?: boolean;
  onError: (message: string | null) => void;
}

export function RoleModelRow(props: RoleModelRowProps) {
  const { role, label, hint, withDim, onError } = props;
  const { t } = useI18n();
  const configService = useConfigService();
  const { customList, templateList } = useAiProvideOptions();

  const [useCloud, setUseCloud] = useState<boolean>(props.useCloud);
  const [provideId, setProvideId] = useState<string>(props.provideId);
  const [dim, setDim] = useState<string>(props.dim === null ? '' : String(props.dim));

  const selectedValue = useCloud ? USE_DEFAULT : provideId;

  const resolveOptions = (): SelectOption[] => {
    const options: SelectOption[] = [{ value: USE_DEFAULT, label: t('set.model.roleUseDefault') }];
    customList.forEach(provide => {
      if (isNotEmpty(provide.id)) {
        options.push({ value: provide.id, label: resolveAiProvideModelLabel(provide, templateList) });
      }
    });
    if (!useCloud && isNotEmpty(provideId) && !options.some(option => option.value === provideId)) {
      options.push({ value: provideId, label: provideId });
    }
    return options;
  };

  const persist = (nextUseCloud: boolean, nextProvideId: string, nextDim: string): void => {
    onError(null);
    const dimValue = Number(nextDim);
    configService.saveRoleModel(role, nextUseCloud, isEmpty(nextProvideId) ? null : nextProvideId, Number.isInteger(dimValue) && dimValue > 0 ? dimValue : null).catch(() => {
      onError(t('common.saveFailed'));
    });
  };

  const handleSelect = (value: string): void => {
    const nextUseCloud = value === USE_DEFAULT;
    const nextProvideId = nextUseCloud ? '' : value;
    setUseCloud(nextUseCloud);
    setProvideId(nextProvideId);
    persist(nextUseCloud, nextProvideId, dim);
  };

  const handleDimBlur = (): void => {
    persist(useCloud, provideId, dim);
  };

  const renderDimRow = (): ReactNode => {
    if (useCloud || withDim !== true) {
      return null;
    }
    return (
      <Box className="tm-set-row">
        <Box className="tm-set-row-main">
          <Typography className="tm-set-row-label">{t('set.model.roleEmbeddingDim')}</Typography>
          <Typography className="tm-set-row-hint">{t('set.model.roleEmbeddingDimHint')}</Typography>
        </Box>
        <Box className="tm-set-row-control">
          <EditField value={dim} onChange={setDim} onBlur={handleDimBlur} placeholder={t('set.model.roleEmbeddingDimPlaceholder')} />
        </Box>
      </Box>
    );
  };

  useEffect(() => {
    setUseCloud(props.useCloud);
    setProvideId(props.provideId);
    setDim(props.dim === null ? '' : String(props.dim));
  }, [props.useCloud, props.provideId, props.dim]);

  return (
    <>
      <Box className="tm-set-row">
        <Box className="tm-set-row-main">
          <Typography className="tm-set-row-label">{label}</Typography>
          <Typography className="tm-set-row-hint">{hint}</Typography>
        </Box>
        <Box className="tm-set-row-control">
          <SelectField value={selectedValue} onChange={handleSelect} options={resolveOptions()} />
        </Box>
      </Box>
      {renderDimRow()}
    </>
  );
}
