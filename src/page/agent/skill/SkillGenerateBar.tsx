import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {generateId} from '@/common/util/IdUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import type {AiProvide} from '@/common/model/AiProvide.ts';
import {AutoAwesome, StopCircleOutlined} from '@mui/icons-material';
import {MarkdownField} from '@/page/agent/common/MarkdownField.tsx';
import {isEmpty, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {ModelSelect} from '@/page/common/components/model/ModelSelect.tsx';
import {SelectField, type SelectOption} from '@/page/common/components/input/SelectField.tsx';
import {BizError, RESULT_CODE, resolveResultMessage} from '@/common/model/resp/ResultModel.ts';
import {useAiAgentApiService, useAiChatService, useAiProvideService} from '@/common/context/useAppHooks.tsx';

interface SkillGenerateBarProps {
    name: string | null;
    content: string | null;
    description: string | null;
    onGenerated: (skill: AiSkill) => void;
    onError: (message: string) => void;
}

export function SkillGenerateBar(props: SkillGenerateBarProps) {
    const {t} = useI18n();
    const aiChatService = useAiChatService();
    const taskIdRef = useRef<string | null>(null);
    const aiProvideService = useAiProvideService();
    const aiAgentApiService = useAiAgentApiService();
    const [chats, setChats] = useState<AiChat[]>([]);
    const [generating, setGenerating] = useState(false);
    const abortRef = useRef<AbortController | null>(null);
    const [chatId, setChatId] = useState<string | null>(null);
    const [provideId, setProvideId] = useState<string | null>(null);
    const [keyPoints, setKeyPoints] = useState<string | null>(null);
    const {name, description, content, onGenerated, onError} = props;

    function sortChats(list: AiChat[]): AiChat[] {
        return [...list].sort((a, b) => b.timestamp - a.timestamp);
    }

    function resolveChatOptions(): SelectOption[] {
        const clearOption: SelectOption = {value: '', label: t('agent.skill.generateNoChat')};
        const chatOptions = chats.map((chat) => {
            return {value: chat.id, label: chat.title};
        });
        return [clearOption, ...chatOptions];
    }

    function resolveKeyPointsInputValue(): string | undefined {
        if (isNotNil(keyPoints)) {
            return keyPoints;
        }
        return undefined;
    }

    function resolveOptionalText(value: string | null): string | null {
        if (!value) {
            return null;
        }
        const trimmed = value.trim();
        if (!trimmed) {
            return null;
        }
        return trimmed;
    }

    function resolveKeyPointsForRequest(): string | null {
        return resolveOptionalText(keyPoints);
    }

    function hasKeyPoints(): boolean {
        return isNotEmpty(resolveKeyPointsForRequest());
    }

    function canGenerate(): boolean {
        if (generating) {
            return false;
        }
        if (isEmpty(provideId)) {
            return false;
        }
        if (isNotEmpty(chatId)) {
            return true;
        }
        return hasKeyPoints();
    }

    function handleChatChange(value: string): void {
        if (!value) {
            setChatId(null);
            return;
        }
        setChatId(value);
    }

    function handleSelectProvide(provide: AiProvide): void {
        if (!provide.id) {
            return;
        }
        setProvideId(provide.id);
        aiProvideService.setSelectedProvide(provide.id, provide.source);
    }

    function handleKeyPointsChange(value: string): void {
        if (!value) {
            setKeyPoints(null);
            return;
        }
        setKeyPoints(value);
    }

    function resolveSkillId(id: string | null | undefined): string | null {
        if (isEmpty(id)) {
            return null;
        }
        return id;
    }

    function resolveGenerateErrorMessage(err: unknown): string {
        if (err instanceof BizError) {
            return resolveResultMessage(err.code);
        }
        if (err instanceof Error) {
            return err.message;
        }
        return t('agent.skill.generateFailed');
    }

    function isCancelledError(err: unknown, signal: AbortSignal): boolean {
        if (signal.aborted) {
            return true;
        }
        if (!(err instanceof BizError)) {
            return false;
        }
        return err.code === RESULT_CODE.CANCELLED;
    }

    async function handleGenerate(): Promise<void> {
        if (!canGenerate()) {
            return;
        }
        if (!provideId) {
            return;
        }
        const taskId = generateId();
        const controller = new AbortController();
        taskIdRef.current = taskId;
        abortRef.current = controller;
        setGenerating(true);
        try {
            const draft = await aiAgentApiService.generateSkill({
                taskId,
                chatId,
                aiProvideId: provideId,
                keyPoints: resolveKeyPointsForRequest(),
                name: resolveOptionalText(name),
                description: resolveOptionalText(description),
                content: resolveOptionalText(content),
            }, controller.signal);
            onGenerated({
                id: resolveSkillId(draft.id),
                sort: draft.sort,
                name: draft.name,
                content: draft.content,
                description: draft.description,
            });
        } catch (err) {
            if (isCancelledError(err, controller.signal)) {
                return;
            }
            onError(resolveGenerateErrorMessage(err));
        } finally {
            taskIdRef.current = null;
            abortRef.current = null;
            setGenerating(false);
        }
    }

    async function handleCancel(): Promise<void> {
        const taskId = taskIdRef.current;
        const controller = abortRef.current;
        if (controller) {
            controller.abort();
        }
        if (!taskId) {
            return;
        }
        try {
            await aiAgentApiService.cancelGenerateSkill({taskId});
        } catch (_err) {
        }
    }

    function handleActionClick(): void {
        if (generating) {
            void handleCancel();
            return;
        }
        void handleGenerate();
    }

    useEffect(() => {
        void aiChatService.selectAll().then((list) => {
            setChats(sortChats(list));
        });
        return () => {
            const controller = abortRef.current;
            if (controller) {
                controller.abort();
            }
        };
    }, [aiChatService]);

    return (
        <Box className="tm-agent-skill-generate">
            <MarkdownField label={t('agent.skill.generateRequirement')} value={resolveKeyPointsInputValue()} onChange={handleKeyPointsChange} placeholder={t('agent.skill.generatePlaceholder')} rows={4} className="tm-agent-skill-generate-md"/>
            <Box className="tm-agent-skill-generate-toolbar">
                <Box className="tm-agent-skill-generate-field">
                    <SelectField label={t('agent.skill.generateChat')} labelWidth={40} value={chatId} onChange={handleChatChange} options={resolveChatOptions()} placeholder={t('agent.skill.generateChatPlaceholder')} menuUp searchable searchPlaceholder={t('agent.skill.generateChatSearch')}/>
                </Box>
                <Box className="tm-agent-skill-generate-field">
                    <ModelSelect variant="row" label={t('agent.skill.generateModel')} labelWidth={40} selectedId={provideId} onSelect={handleSelectProvide} autoSelect/>
                </Box>
                <ButtonBase disableRipple disabled={!generating && !canGenerate()} onClick={handleActionClick} aria-label={generating ? t('agent.skill.generateCancel') : t('agent.skill.generateStart')} className={generating ? 'tm-agent-skill-generate-btn is-generating' : 'tm-agent-skill-generate-btn'}>
                    {generating ? <StopCircleOutlined sx={{fontSize: 16}}/> : <AutoAwesome sx={{fontSize: 16}}/>}
                    <Typography component="span" className="tm-agent-skill-generate-btn-text">{generating ? t('agent.skill.generateStop') : t('agent.skill.generateStart')}</Typography>
                </ButtonBase>
            </Box>
        </Box>
    );
}
