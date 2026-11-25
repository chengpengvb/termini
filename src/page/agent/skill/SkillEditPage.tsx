import type {ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {usePaneContext} from '@/common/components/tab';
import {createElement, useEffect, useState} from 'react';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil.ts';
import {SkillIcon} from '@/assets/icon/components/SkillIcon.tsx';
import {MarkdownField} from '@/page/agent/common/MarkdownField.tsx';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {Box, Button, CircularProgress, Typography} from '@mui/material';
import {SkillGenerateBar} from '@/page/agent/skill/SkillGenerateBar.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';
import {BizError, resolveResultMessage} from '@/common/model/resp/ResultModel.ts';
import {AI_SKILL_DESCRIPTION_MAX_LENGTH, type AiSkill} from '@/common/model/AiSkill.ts';
import {AlertDialog, type AlertColor} from '@/common/components/dialog/AlertDialog.tsx';
import {useAiSkillService, useTabManagerService} from '@/common/context/useAppHooks.tsx';

interface SkillEditPageProps {
    skill: AiSkill | null;
    onSaved: () => void;
}

function createEmptySkill(): AiSkill {
    return {
        id: null,
        sort: 100,
        name: null,
        content: null,
        description: null,
    };
}

function resolveNormalizedSkill(skill: AiSkill): AiSkill {
    return {
        id: skill.id,
        sort: skill.sort,
        name: isEmpty(skill.name) ? null : skill.name.trim(),
        content: isEmpty(skill.content) ? null : skill.content.trim(),
        description: isEmpty(skill.description) ? null : skill.description.trim(),
    };
}

export function SkillEditPage(props: SkillEditPageProps) {
    const {t, tf} = useI18n();
    const {skill, onSaved} = props;
    const {paneId} = usePaneContext();
    const [saving, setSaving] = useState(false);
    const aiSkillService = useAiSkillService();
    const tabManagerService = useTabManagerService();
    const [alert, setAlert] = useState<{ type: AlertColor; message: string } | null>(null);
    const [form, setForm] = useState<AiSkill>(() => {
        if (skill) {
            return {...skill};
        }
        return createEmptySkill();
    });
    const [snapshot, setSnapshot] = useState<AiSkill>(() => {
        if (skill) {
            return {...skill};
        }
        return createEmptySkill();
    });

    function isEditMode(): boolean {
        if (isNotEmpty(form.id)) {
            return true;
        }
        return Boolean(skill);
    }

    function resolveTitle(): string {
        if (isEditMode()) {
            return t('agent.skill.editTitle');
        }
        return t('agent.skill.newTitle');
    }

    function resolveNameValue(): string | null {
        return form.name;
    }

    function resolveDescriptionValue(): string | null {
        return form.description;
    }

    function resolveContentValue(): string | undefined {
        if (!form.content) {
            return undefined;
        }
        return form.content;
    }

    function resolveDescriptionForSave(): string | null {
        if (isEmpty(form.description)) {
            return null;
        }
        return form.description.trim();
    }

    function resolveTabTitle(): string {
        if (isNotEmpty(form.name)) {
            return form.name.trim();
        }
        return resolveTitle();
    }

    function isDirty(): boolean {
        const current = resolveNormalizedSkill(form);
        const baseline = resolveNormalizedSkill(snapshot);
        if (current.name !== baseline.name) {
            return true;
        }
        if (current.description !== baseline.description) {
            return true;
        }
        if (current.content !== baseline.content) {
            return true;
        }
        return current.sort !== baseline.sort;

    }

    function canSave(): boolean {
        if (saving) {
            return false;
        }
        if (isEmpty(form.name)) {
            return false;
        }
        if (isEmpty(form.content)) {
            return false;
        }
        return isDirty();
    }

    function renderSaveStartIcon(): ReactNode {
        if (!saving) {
            return null;
        }
        return <CircularProgress size={14} color="inherit"/>;
    }

    function syncTabBar(title: string): void {
        tabManagerService.updatePaneBar(paneId, {
            canClose: true,
            title,
            icon: createElement(SkillIcon, {fontSize: 16}),
        });
    }

    function handleClose(): void {
        tabManagerService.closePane(paneId);
    }

    function handleFormTextChange(field: 'name' | 'description' | 'content', value: string): void {
        if (field === 'description' && value.length > AI_SKILL_DESCRIPTION_MAX_LENGTH) {
            setAlert({type: 'error', message: tf('agent.skill.descriptionMax', {n: AI_SKILL_DESCRIPTION_MAX_LENGTH})});
            return;
        }
        if (!value) {
            setForm({...form, [field]: null});
            return;
        }
        setForm({...form, [field]: value});
    }

    function handleAlertClose(): void {
        setAlert(null);
    }

    function handleGenerated(draft: AiSkill): void {
        if (draft.description !== null && draft.description.length > AI_SKILL_DESCRIPTION_MAX_LENGTH) {
            setAlert({type: 'error', message: tf('agent.skill.descriptionMax', {n: AI_SKILL_DESCRIPTION_MAX_LENGTH})});
            setForm({
                ...form,
                name: draft.name,
                content: draft.content,
                sort: draft.sort,
            });
            return;
        }
        setForm({
            ...form,
            name: draft.name,
            content: draft.content,
            description: draft.description,
            sort: draft.sort,
        });
    }

    function handleGenerateError(message: string): void {
        setAlert({type: 'error', message});
    }

    async function handleSave(): Promise<void> {
        if (!canSave()) {
            return;
        }
        if (isEmpty(form.name)) {
            return;
        }
        if (isEmpty(form.content)) {
            return;
        }
        setSaving(true);
        setAlert(null);
        try {
            const payload: AiSkill = {
                ...form,
                name: form.name.trim(),
                content: form.content.trim(),
                description: resolveDescriptionForSave(),
            };
            let saved: AiSkill;
            if (isNotEmpty(form.id)) {
                saved = await aiSkillService.update(payload);
            } else {
                saved = await aiSkillService.insert(payload);
            }
            setForm(saved);
            setSnapshot(saved);
            onSaved();
            setAlert({type: 'success', message: t('agent.skill.saveSuccess')});
        } catch (err) {
            if (err instanceof BizError) {
                setAlert({type: 'error', message: resolveResultMessage(err.code)});
                return;
            }
            if (err instanceof Error) {
                setAlert({type: 'error', message: err.message});
                return;
            }
            setAlert({type: 'error', message: t('agent.skill.saveFailed')});
        } finally {
            setSaving(false);
        }
    }

    useEffect(() => {
        syncTabBar(resolveTabTitle());
    }, [form.id, form.name, paneId, skill, tabManagerService]);

    return (
        <Box className="tm-agent-skill-edit">
            <Box className="tm-agent-skill-edit-topbar tm-page-column">
                <Typography component="h1" className="tm-agent-skill-edit-title">{resolveTitle()}</Typography>
                <Box className="tm-agent-skill-edit-actions">
                    <Button disableRipple onClick={handleClose} className="tm-btn tm-btn-secondary">{t('common.close')}</Button>
                    <Button disableRipple disabled={!canSave()} onClick={() => {
                        void handleSave();
                    }} startIcon={renderSaveStartIcon()} className="tm-btn tm-btn-primary">
                        {t('common.save')}
                    </Button>
                </Box>
            </Box>
            <Box className="tm-agent-skill-edit-body">
                <Box className="tm-agent-page tm-page-column tm-agent-skill-edit-stage">
                    <Box className="tm-agent-section">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.skill.basicInfo')}</Typography>
                        </Box>
                        <Box className="tm-agent-skill-edit-fields">
                            <EditField label={t('agent.skill.name')} value={resolveNameValue()} onChange={(value) => handleFormTextChange('name', value)} placeholder={t('agent.skill.namePlaceholder')} autoFocus/>
                            <TextareaField label={t('agent.skill.description')} value={resolveDescriptionValue()} onChange={(value) => handleFormTextChange('description', value)} placeholder={t('agent.skill.descriptionPlaceholder')} minRows={3} maxRows={3}
                                           maxLength={AI_SKILL_DESCRIPTION_MAX_LENGTH}/>
                        </Box>
                    </Box>
                    <Box className="tm-agent-section is-content">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.skill.content')}</Typography>
                        </Box>
                        <Box className="tm-agent-skill-edit-content">
                            <MarkdownField value={resolveContentValue()} onChange={(value) => handleFormTextChange('content', value)} placeholder={t('agent.skill.contentPlaceholder')} rows={18} className="tm-agent-skill-edit-md"/>
                        </Box>
                    </Box>
                    <Box className="tm-agent-section">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.skill.refine')}</Typography>
                        </Box>
                        <SkillGenerateBar name={form.name} description={form.description} content={form.content} onGenerated={handleGenerated} onError={handleGenerateError}/>
                    </Box>
                </Box>
            </Box>
            <AlertDialog duration={1500} open={alert !== null} handleClose={handleAlertClose} message={alert === null ? null : alert.message} type={alert === null ? 'error' : alert.type}/>
        </Box>
    );
}
