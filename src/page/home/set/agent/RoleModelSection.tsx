import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {MODEL_ROLE, type Config} from '@/common/model/Config.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {RoleModelRow} from '@/page/home/set/agent/RoleModelRow.tsx';

export function RoleModelSection() {
    const {t} = useI18n();
    const configService = useConfigService();

    const [config, setConfig] = useState<Config | null>(null);
    const [error, setError] = useState<string | null>(null);

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    const renderRows = (): ReactNode => {
        if (config === null) {
            return null;
        }
        return (
            <>
                <RoleModelRow role={MODEL_ROLE.VISION} label={t('set.model.roleVision')} hint={t('set.model.roleVisionHint')} useCloud={config.vision_use_cloud !== false} provideId={config.vision_local_ai_provide_id ?? ''} dim={null} onError={setError}/>
                <RoleModelRow role={MODEL_ROLE.IMAGE_GEN} label={t('set.model.roleImageGen')} hint={t('set.model.roleImageGenHint')} useCloud={config.image_gen_use_cloud !== false} provideId={config.image_gen_local_ai_provide_id ?? ''} dim={null} onError={setError}/>
                <RoleModelRow role={MODEL_ROLE.EMBEDDING} label={t('set.model.roleEmbedding')} hint={t('set.model.roleEmbeddingHint')} useCloud={config.embedding_use_cloud !== false} provideId={config.embedding_local_ai_provide_id ?? ''} dim={config.embedding_local_dim ?? null} withDim onError={setError}/>
                <RoleModelRow role={MODEL_ROLE.ASSIST} label={t('set.model.roleAssist')} hint={t('set.model.roleAssistHint')} useCloud={config.assist_use_cloud !== false} provideId={config.assist_local_ai_provide_id ?? ''} dim={null} onError={setError}/>
            </>
        );
    };

    useEffect(() => {
        configService.selectConfig().then(setConfig).catch(() => {
            setError(t('common.loadFailed'));
        });
    }, [configService, t]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.model.roleTitle')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.model.roleDesc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                {renderRows()}
            </Box>
        </Box>
    );
}
