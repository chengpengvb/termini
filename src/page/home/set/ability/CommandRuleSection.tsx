import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {Box, Button, Typography} from '@mui/material';
import {useCommandRuleService} from '@/common/context/useAppHooks.tsx';
import {CommandRuleDialog} from '@/page/home/set/ability/CommandRuleDialog.tsx';
import {formatRulePattern, RULE_DECISION, type CommandRule} from '@/common/model/CommandRule.ts';

export function CommandRuleSection() {
    const {t} = useI18n();
    const commandRuleService = useCommandRuleService();
    const [busy, setBusy] = useState<boolean>(false);
    const [rules, setRules] = useState<CommandRule[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);

    const resolveDecisionLabel = (value: number): string => {
        if (value === RULE_DECISION.ALLOW) {
            return t('set.commandRule.allow');
        }
        if (value === RULE_DECISION.FORBIDDEN) {
            return t('set.commandRule.forbidden');
        }
        return t('set.commandRule.prompt');
    };

    const resolveDecisionClass = (value: number): string => {
        if (value === RULE_DECISION.FORBIDDEN) {
            return 'tm-set-index-dir-state is-failed';
        }
        return 'tm-set-index-dir-state';
    };

    const resolveRuleText = (rule: CommandRule): string => {
        const parts = [formatRulePattern(rule)];
        if (rule.device_id !== '') {
            parts.push(rule.device_id);
        }
        if (rule.justification !== '') {
            parts.push(rule.justification);
        }
        return parts.join(' · ');
    };

    const refresh = (): void => {
        commandRuleService.selectAll().then(setRules).catch(() => {
            setError(t('common.loadFailed'));
        });
    };

    const handleSaved = (): void => {
        setDialogOpen(false);
        refresh();
    };

    const handleRemove = (rule: CommandRule): void => {
        setBusy(true);
        setError(null);
        commandRuleService.deleteById(rule.id).then(refresh).catch(() => {
            setError(t('common.saveFailed'));
        }).finally(() => {
            setBusy(false);
        });
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    const renderRules = (): ReactNode => {
        if (rules.length === 0) {
            return (
                <Box className="tm-set-row">
                    <Typography className="tm-set-row-hint">{t('set.commandRule.empty')}</Typography>
                </Box>
            );
        }
        return rules.map(rule => (
            <Box key={rule.id} className="tm-set-row">
                <Box className="tm-set-row-main">
                    <Typography className="tm-set-index-dir-path" title={resolveRuleText(rule)}>
                        {resolveRuleText(rule)}
                    </Typography>
                    <Typography className={resolveDecisionClass(rule.decision)}>{resolveDecisionLabel(rule.decision)}</Typography>
                </Box>
                <Box className="tm-set-index-dir-actions">
                    <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" disabled={busy} onClick={() => handleRemove(rule)}>
                        {t('set.commandRule.remove')}
                    </Button>
                </Box>
            </Box>
        ));
    };

    useEffect(() => {
        refresh();
    }, [commandRuleService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.commandRule.title')}</Typography>
                    <Typography className="tm-set-card-desc">{t('set.commandRule.desc')}</Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                    <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" disabled={busy} onClick={() => setDialogOpen(true)}>
                        {t('set.commandRule.add')}
                    </Button>
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                {renderRules()}
            </Box>
            <CommandRuleDialog open={dialogOpen} onSaved={handleSaved} onClose={() => setDialogOpen(false)}/>
        </Box>
    );
}
