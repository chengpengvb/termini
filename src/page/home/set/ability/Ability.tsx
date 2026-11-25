import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { AlertSection } from '@/page/home/set/ability/AlertSection.tsx';
import { SuggestSection } from '@/page/home/set/ability/SuggestSection.tsx';
import { DiscoverSection } from '@/page/home/set/ability/DiscoverSection.tsx';
import { AuditPolicySection } from '@/page/home/set/ability/AuditPolicySection.tsx';
import { CommandRuleSection } from '@/page/home/set/ability/CommandRuleSection.tsx';

export function Ability() {
  const { t } = useI18n();
  const mobile = usePlatformMobile();
  const jobAbility = !mobile;

  return (
    <Box className="tm-set-content">
      <Box className="tm-set-content-header">
        <Typography className="tm-set-content-title">{t('set.ability.title')}</Typography>
      </Box>
      <SuggestSection />
      <AuditPolicySection />
      <CommandRuleSection />
      {jobAbility ? <AlertSection /> : null}
      {jobAbility ? <DiscoverSection /> : null}
    </Box>
  );
}
