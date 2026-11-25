import { useI18n } from '@/common/i18n/useI18n.ts';
import { Check, Search } from '@mui/icons-material';
import { useEffect, useRef, useState } from 'react';
import { Box, InputBase, ButtonBase, Typography } from '@mui/material';
import type { RespProvider } from '@/common/model/resp/RespProvider.ts';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { AI_PROVIDE_SOURCE, resolveAiProvideModelLabel, resolveAiProvideProviderName, resolveAiProvideRateLabel, type AiProvide } from '@/common/model/AiProvide.ts';

interface ModelSelectMenuProps {
  mobileViewport: boolean;
  cloudList: AiProvide[];
  customList: AiProvide[];
  selected: AiProvide | null;
  refreshing: boolean;
  templateList: RespProvider[];
  onPick: (provide: AiProvide) => void;
}

export function ModelSelectMenu(props: ModelSelectMenuProps) {
  const { t } = useI18n();
  const { selected, cloudList, customList, templateList, refreshing, mobileViewport, onPick } = props;
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [keyword, setKeyword] = useState<string | null>(null);

  function filterByKeyword(list: AiProvide[]): AiProvide[] {
    if (isEmpty(keyword)) {
      return list;
    }
    const value = keyword.toLowerCase();
    return list.filter(item => {
      if (isNotEmpty(item.model) && item.model.toLowerCase().includes(value)) {
        return true;
      }
      return resolveAiProvideProviderName(item, templateList).toLowerCase().includes(value);
    });
  }

  function resolveKeywordValue(): string {
    if (isNotEmpty(keyword)) {
      return keyword;
    }
    return '';
  }

  function handleKeywordChange(raw: string): void {
    if (isEmpty(raw)) {
      setKeyword(null);
      return;
    }
    setKeyword(raw);
  }

  function resolveCloudFiltered(): AiProvide[] {
    return filterByKeyword(cloudList);
  }

  function resolveCustomFiltered(): AiProvide[] {
    return filterByKeyword(customList);
  }

  function renderGroup(label: string | null, list: AiProvide[]) {
    if (list.length === 0) {
      return null;
    }
    return (
      <>
        {isEmpty(label) ? null : <Typography sx={{ px: 1.25, pt: 0.75, pb: 0.25, fontSize: 11, fontWeight: 700, letterSpacing: 0.4, color: 'var(--tm-text-tertiary)' }}>{label}</Typography>}
        {list.map(provide => {
          const isSelected = isNotNil(selected) && selected.id === provide.id;
          const rateLabel = resolveAiProvideRateLabel(provide);
          const modelLabel = resolveAiProvideModelLabel(provide, templateList);
          const providerLabel = resolveAiProvideProviderName(provide, templateList);
          const isCloud = provide.source === AI_PROVIDE_SOURCE.CLOUD;
          return (
            <ButtonBase key={`${provide.source}:${provide.id}`} disableRipple onClick={() => onPick(provide)} className={isSelected ? 'tm-agent-model-option is-selected' : 'tm-agent-model-option'}>
              <Box className="tm-agent-model-option-namewrap">
                <Typography className="tm-agent-model-option-name" title={modelLabel}>
                  {modelLabel}
                </Typography>
                {!isCloud && rateLabel !== null ? (
                  <Typography className="tm-agent-model-option-rate" title={t('common.modelRateHint')}>
                    {rateLabel}
                  </Typography>
                ) : null}
              </Box>
              {isCloud ? (
                rateLabel !== null ? (
                  <Typography className="tm-agent-model-option-provider">{rateLabel}</Typography>
                ) : null
              ) : (
                <Typography className="tm-agent-model-option-provider" title={providerLabel}>
                  {providerLabel}
                </Typography>
              )}
              <Box className="tm-agent-model-option-check">{isSelected ? <Check sx={{ fontSize: 14 }} /> : null}</Box>
            </ButtonBase>
          );
        })}
      </>
    );
  }

  useEffect(() => {
    searchInputRef.current?.focus({ preventScroll: true });
  }, []);

  return (
    <Box className={mobileViewport ? 'tm-agent-model-menu is-sheet' : 'tm-agent-model-menu'}>
      {mobileViewport ? null : <Typography sx={{ px: 1.25, pt: 0.25, fontSize: 11, fontWeight: 700, letterSpacing: 0.4, color: 'var(--tm-text-tertiary)' }}>{t('common.selectModel')}</Typography>}
      <Box className={mobileViewport ? 'tm-action-sheet-search' : 'tm-agent-mount-search-row'}>
        <Search className={mobileViewport ? undefined : 'tm-agent-model-search-icon'} sx={mobileViewport ? { fontSize: 18, color: 'var(--tm-text-tertiary)', flexShrink: 0 } : undefined} />
        <InputBase
          inputRef={searchInputRef}
          value={resolveKeywordValue()}
          placeholder={t('common.searchModel')}
          onChange={e => handleKeywordChange(e.target.value)}
          className={mobileViewport ? undefined : 'tm-agent-model-search'}
          sx={mobileViewport ? { flex: 1, fontSize: 16, color: 'var(--tm-text-primary)', '& input::placeholder': { color: 'var(--tm-text-tertiary)', opacity: 1 } } : undefined}
        />
      </Box>
      <Box className="tm-agent-model-option-list">
        {renderGroup(null, resolveCloudFiltered())}
        {renderGroup('Custom', resolveCustomFiltered())}
        {resolveCloudFiltered().length + resolveCustomFiltered().length > 0 ? null : <Typography className="tm-agent-model-empty">{refreshing ? t('common.modelLoading') : t('common.noMatchingModel')}</Typography>}
      </Box>
    </Box>
  );
}
