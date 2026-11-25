import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {useAiProvideService} from '@/common/context/useAppHooks.tsx';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';
import {DeleteOutlined, DriveFileRenameOutline, Hub} from '@mui/icons-material';
import {AI_PROVIDE_SOURCE, resolveAiProvideModelLabel, resolveAiProvideProviderName, type AiProvide} from '@/common/model/AiProvide.ts';

export function SavedModel(props: {templateList: RespProvider[]; onAdd: () => void; onEdit: (provide: AiProvide) => void}) {
    const {templateList, onAdd, onEdit} = props;
    const {t} = useI18n();
    const aiProvideService = useAiProvideService();
    const [provideList, setProvideList] = useState<AiProvide[]>([]);

    const filterCustomProvides = (list: AiProvide[]): AiProvide[] => {
        return list.filter(item => item.source === AI_PROVIDE_SOURCE.CUSTOM);
    };

    const reloadProvideList = (): void => {
        aiProvideService.selectAll().then(list => setProvideList(filterCustomProvides(list)));
    };

    const resolveProviderName = (provide: AiProvide): string => {
        return resolveAiProvideProviderName(provide, templateList);
    };

    const resolveModelTitle = (provide: AiProvide): string => {
        return resolveAiProvideModelLabel(provide, templateList);
    };

    const handleEdit = (provide: AiProvide): void => {
        onEdit(provide);
    };

    const handleDelete = (id: string | undefined): void => {
        if (isEmpty(id)) {
            return;
        }
        aiProvideService.deleteById(id).then(reloadProvideList);
    };

    const renderBody = (): ReactNode => {
        if (provideList.length === 0) {
            return (
                <Box className="tm-empty-state">
                    <Box className="tm-empty-state-card">
                        <Box className="tm-empty-state-icon">
                            <Hub className="tm-empty-state-icon-svg"/>
                        </Box>
                        <Typography className="tm-empty-state-title" onClick={onAdd}>{t('set.model.emptyTitle')}</Typography>
                        <Typography className="tm-empty-state-message">{t('set.model.emptyMessage')}</Typography>
                    </Box>
                </Box>
            );
        }
        return (
            <Box className="tm-set-saved-list">
                {provideList.map(provide => (
                    <Box key={provide.id} className="tm-set-saved-item">
                        <Box className="tm-set-saved-item-main">
                            <Typography className="tm-set-saved-item-title">{resolveModelTitle(provide)}</Typography>
                            <Typography className="tm-set-saved-item-sub">{resolveProviderName(provide)}</Typography>
                        </Box>
                        <Box className="tm-set-saved-item-actions">
                            <ButtonBase disableRipple aria-label={t('common.edit')} className="tm-set-saved-item-action" onClick={() => handleEdit(provide)}>
                                <DriveFileRenameOutline sx={{fontSize: 16}}/>
                            </ButtonBase>
                            <ButtonBase disableRipple aria-label={t('common.delete')} className="tm-set-saved-item-action is-danger" onClick={() => handleDelete(provide.id)}>
                                <DeleteOutlined sx={{fontSize: 16}}/>
                            </ButtonBase>
                        </Box>
                    </Box>
                ))}
            </Box>
        );
    };

    useEffect(() => {
        reloadProvideList();
        const off = aiProvideService.addAiProvideListChangeListener(list => setProvideList(filterCustomProvides(list)));
        return () => {
            off();
        };
    }, [aiProvideService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.model.savedTitle')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.model.savedDesc')}
                    </Typography>
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                {renderBody()}
            </Box>
        </Box>
    );
}
