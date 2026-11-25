import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { MergeField } from '@/page/common/dto/account/MergeTypes.ts';

function formatValue(value: unknown, emptyLabel: string): string {
  if (value === null || value === undefined) {
    return emptyLabel;
  }
  if (value instanceof Object) {
    return JSON.stringify(value);
  }
  return String(value);
}

export function MergeCompare(props: { fields: MergeField[]; local: Record<string, unknown>; cloud: Record<string, unknown> }) {
  const { fields, local, cloud } = props;
  const { t } = useI18n();

  const isDiff = (key: string): boolean => {
    return JSON.stringify(local[key]) !== JSON.stringify(cloud[key]);
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', border: '1px solid var(--tm-border-subtle)', borderRadius: 1.5, overflow: 'hidden' }}>
      <Box sx={{ display: 'grid', gridTemplateColumns: '92px 1fr 1fr', gap: 1, px: 1.25, py: 0.75, bgcolor: 'var(--tm-bg-hover)' }}>
        <Typography sx={{ fontSize: 12, fontWeight: 700, color: 'var(--tm-text-secondary)' }}>{t('account.merge.fieldHeader')}</Typography>
        <Typography sx={{ fontSize: 12, fontWeight: 700, color: 'var(--tm-diff-del-fg)' }}>{t('account.merge.localHeader')}</Typography>
        <Typography sx={{ fontSize: 12, fontWeight: 700, color: 'var(--tm-diff-add-fg)' }}>{t('account.merge.cloudHeader')}</Typography>
      </Box>
      {fields.map(field => (
        <Box key={field.key} sx={{ display: 'grid', gridTemplateColumns: '92px 1fr 1fr', gap: 1, px: 1.25, py: 0.5, alignItems: 'start' }}>
          <Typography sx={{ fontSize: 12, color: 'var(--tm-text-secondary)', pt: 0.25 }}>{t(field.labelKey)}</Typography>
          <Box sx={{ fontSize: 12, fontFamily: 'var(--tm-font-mono)', overflowWrap: 'anywhere', px: 0.75, py: 0.25, borderRadius: 1, color: isDiff(field.key) ? 'var(--tm-diff-del-fg)' : 'var(--tm-text-primary)', bgcolor: isDiff(field.key) ? 'var(--tm-diff-del-bg)' : 'transparent' }}>
            {isDiff(field.key) ? '− ' : ''}
            {formatValue(local[field.key], t('account.merge.empty'))}
          </Box>
          <Box sx={{ fontSize: 12, fontFamily: 'var(--tm-font-mono)', overflowWrap: 'anywhere', px: 0.75, py: 0.25, borderRadius: 1, color: isDiff(field.key) ? 'var(--tm-diff-add-fg)' : 'var(--tm-text-primary)', bgcolor: isDiff(field.key) ? 'var(--tm-diff-add-bg)' : 'transparent' }}>
            {isDiff(field.key) ? '+ ' : ''}
            {formatValue(cloud[field.key], t('account.merge.empty'))}
          </Box>
        </Box>
      ))}
    </Box>
  );
}
