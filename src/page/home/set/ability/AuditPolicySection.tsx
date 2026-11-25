import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';

export function AuditPolicySection() {
    const {t} = useI18n();
    const configService = useConfigService();

    const [policy, setPolicy] = useState<string>('');
    const [error, setError] = useState<string | null>(null);

    const persist = (): void => {
        setError(null);
        configService.saveAuditPolicy(policy.trim() === '' ? null : policy).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            setPolicy(config?.audit_policy ?? '');
        }).catch(() => {
            setPolicy('');
        });
    }, [configService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.auditPolicy.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.auditPolicy.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <Box className="tm-set-row is-column">
                    <TextareaField value={policy} onChange={setPolicy} onBlur={persist} placeholder={t('set.auditPolicy.placeholder')} expandable language="markdown" minRows={2} maxRows={6}/>
                </Box>
            </Box>
        </Box>
    );
}
