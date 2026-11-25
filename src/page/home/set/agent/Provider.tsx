import {Add} from '@mui/icons-material';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import type {AiProvide} from '@/common/model/AiProvide.ts';
import {SavedModel} from '@/page/home/set/agent/SavedModel.tsx';
import {useAiProvideService} from '@/common/context/useAppHooks.tsx';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';
import {AddModelDialog} from '@/page/home/set/agent/AddModelDialog.tsx';
import {RoleModelSection} from '@/page/home/set/agent/RoleModelSection.tsx';

export function Provider() {
    const {t} = useI18n();
    const aiProvideService = useAiProvideService();
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);
    const [templateList, setTemplateList] = useState<RespProvider[]>([]);
    const [editingProvide, setEditingProvide] = useState<AiProvide | null>(null);

    const handleOpenAdd = (): void => {
        setEditingProvide(null);
        setDialogOpen(true);
    };

    const handleOpenEdit = (provide: AiProvide): void => {
        setEditingProvide(provide);
        setDialogOpen(true);
    };

    const handleCloseDialog = (): void => {
        setDialogOpen(false);
        setEditingProvide(null);
    };

    useEffect(() => {
        aiProvideService.listTemplateProvider().then(setTemplateList);
    }, [aiProvideService]);

    return (
        <Box className="tm-set-content">
            <Box className="tm-set-content-inner">
                <Box className="tm-set-content-header">
                    <Typography className="tm-set-content-title">{t('set.model.title')}</Typography>
                    <ButtonBase disableRipple className="tm-action-btn tm-action-btn-primary tm-set-add-btn" onClick={handleOpenAdd}>
                        <Add className="tm-set-add-btn-icon"/>
                        <Typography component="span" className="tm-set-add-btn-label">{t('set.model.add')}</Typography>
                    </ButtonBase>
                </Box>
                <SavedModel templateList={templateList} onAdd={handleOpenAdd} onEdit={handleOpenEdit}/>
                <RoleModelSection/>
            </Box>
            <AddModelDialog open={dialogOpen} editing={editingProvide} templateList={templateList} onClose={handleCloseDialog}/>
        </Box>
    );
}
