import {Close} from '@mui/icons-material';
import type {Editor} from '@tiptap/react';
import {EditorContent} from '@tiptap/react';
import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useAgentRun} from '@/page/common/hooks/useAgentRun.ts';
import {useKeepAwake} from '@/page/common/hooks/useKeepAwake.ts';
import {InputFrame} from '@/page/agent/main/input/InputFrame.tsx';
import {InputHint} from '@/page/agent/main/input/hint/InputHint.tsx';
import type {AttachedImage} from '@/page/common/util/imageAttach.ts';
import {AlertDialog} from '@/common/components/dialog/AlertDialog.tsx';
import {useInlineRevert} from '@/page/common/hooks/useInlineRevert.ts';
import {useMessageQueue} from '@/page/common/hooks/useMessageQueue.ts';
import {useShellDaemons} from '@/page/common/hooks/useShellDaemons.ts';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {useChatSend} from '@/page/agent/main/input/send/useChatSend.ts';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import {Images} from '@/page/agent/main/input/components/image/Images.tsx';
import {usePlatformNativeMobile} from '@/common/context/usePlatformMobile';
import {Ref, useEffect, useImperativeHandle, useRef, useState} from 'react';
import {Toolbar} from '@/page/agent/main/input/components/tool/Toolbar.tsx';
import {useInputHints} from '@/page/agent/main/input/hint/useInputHints.ts';
import type {AgentQuestionAnswer} from '@/common/model/req/event/WsAgent.ts';
import {serializeEditorMessage} from '@/page/common/util/reference/editor.ts';
import {DropPaste} from '@/page/agent/main/input/components/drop/DropPaste.tsx';
import {useInputEditor} from '@/page/agent/main/input/editor/useInputEditor.ts';
import {isEmpty, isNil, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {QueuedMessages} from '@/page/agent/main/input/components/queue/QueuedMessages.tsx';
import {useReferenceDropZone} from '@/page/agent/main/input/components/drop/useReferenceDropZone.ts';
import {BackgroundTerminals} from '@/page/agent/main/input/components/daemon/BackgroundTerminals.tsx';
import {resolveDefaultThinkingLevel, resolveThinkingLevelNames, type AiProvide} from '@/common/model/AiProvide.ts';
import {useAccountService, useAgentService, useAiProvideService, useAiShareConsentService, useConfirmModeService} from '@/common/context/useAppHooks.tsx';

export interface InputBottomHandle {
    restoreDraft: (message: string) => void;
    applyReference: (ref: ChatReference) => void;
    answerQuestion: (answer: AgentQuestionAnswer) => void;
    beginRevert: (itemId: string, message: string) => void;
}

interface InputBottomProps {
    sessionId: string;
    chatId: string | null;
    onReverted: () => void;
    ref?: Ref<InputBottomHandle>;
    setChatId: (chatId: string | null) => void;
    onRevertChange?: (itemId: string | null) => void;
}

export function InputBottom(props: InputBottomProps) {
    const {t} = useI18n();
    const maxImageCount = 6;
    const chatIdRef = useRef(props.chatId);
    const agentService = useAgentService();
    const accountService = useAccountService();
    const nativeMobile = usePlatformNativeMobile();
    const aiProvideService = useAiProvideService();
    const confirmModeService = useConfirmModeService();
    const handleSendRef = useRef<() => void>(() => {
    });
    const selectedRef = useRef<AiProvide | null>(null);
    const panelRef = useRef<HTMLDivElement | null>(null);
    const wrapRef = useRef<HTMLDivElement | null>(null);
    const thinkingLevelRef = useRef<string | null>(null);
    const [shareAsking, setShareAsking] = useState(false);
    const aiShareConsentService = useAiShareConsentService();
    const sharePendingRef = useRef<(() => void) | null>(null);
    const [dropOver, setDropOver] = useState<boolean>(false);
    const [images, setImages] = useState<AttachedImage[]>([]);
    const [selected, setSelected] = useState<AiProvide | null>(null);
    const [loginAlert, setLoginAlert] = useState<string | null>(null);
    const [thinkingLevel, setThinkingLevel] = useState<string | null>(null);
    const {chatId, setChatId, sessionId, onReverted, onRevertChange, ref} = props;
    const [confirmMode, setConfirmMode] = useState<number>(confirmModeService.getConfirmMode());

    const confirmModeRef = useRef<number>(confirmMode);
    const thinkingLevels = resolveThinkingLevelNames(selected);

    chatIdRef.current = chatId;
    selectedRef.current = selected;
    confirmModeRef.current = confirmMode;
    thinkingLevelRef.current = thinkingLevel;

    const input = useInputEditor({
        chatId,
        chatIdRef,
        onArrow: handleArrow,
        onEditorUpdate: handleEditorUpdate,
        onSubmit: () => handleSendRef.current(),
    });

    const run = useAgentRun({
        chatId,
        sessionId,
        chatIdRef,
        onRunDone: handleRunDone,
        onRunAborted: handleRunAborted,
    });

    const daemons = useShellDaemons(chatId);

    const send = useChatSend({
        chatId,
        chatIdRef,
        images,
        selected,
        sessionId,
        setChatId,
        setImages,
        confirmMode,
        thinkingLevel,
        running: run.running,
    });

    const revert = useInlineRevert({
        chatId,
        wrapRef,
        chatIdRef,
        onReverted,
        onRevertChange,
        setHasAny: input.setHasAny,
        sendChat: send.handleChat,
        resolveEditor: input.resolveEditor,
        onBeginRevert: cancelQueueEditForRevert,
    });

    const queueApi = useMessageQueue({
        chatId,
        images,
        sessionId,
        chatIdRef,
        setImages,
        selectedRef,
        confirmModeRef,
        thinkingLevelRef,
        running: run.running,
        onStop: run.handleStop,
        setHasAny: input.setHasAny,
        resolveEditor: input.resolveEditor,
        onCancelRevert: revert.cancelRevert,
        revertItemId: revert.revertItemId,
        isRevertingRef: revert.isRevertingRef,
    });

    const hints = useInputHints({resolveEditor: input.resolveEditor, applyReference: input.applyReference});

    function handleEditorUpdate(target: Editor): void {
        hints.update(target);
    }

    function handleArrow(delta: number): boolean {
        return hints.moveActive(delta);
    }

    function cancelQueueEditForRevert(): void {
        queueApi.cancelQueueEdit();
    }

    function handleRunDone(questionPending: boolean): void {
        if (queueApi.consumeForceSend()) {
            return;
        }
        if (questionPending) {
            return;
        }
        queueApi.advanceQueue();
    }

    function handleRunAborted(): void {
        queueApi.consumeForceSend();
    }

    function canSend(): boolean {
        if (!selected) {
            return false;
        }
        if (images.length > 0) {
            return true;
        }
        return input.hasAny;
    }

    function handleAddImages(added: AttachedImage[]): void {
        setImages(prev => [...prev, ...added].slice(0, maxImageCount));
    }

    function handleRemoveImage(id: string): void {
        setImages(prev => prev.filter(item => item.id !== id));
    }

    function resolveSelectedId(): string | null {
        if (selected && isNotEmpty(selected.id)) {
            return selected.id;
        }
        return null;
    }

    function handleSelectModel(next: AiProvide): void {
        if (!next.id) {
            return;
        }
        aiProvideService.setSelectedProvide(next.id, next.source);
        setSelected(next);
        const names = resolveThinkingLevelNames(next);
        const remembered = aiProvideService.getThinkingLevel(next.id);
        setThinkingLevel(isNotNil(remembered) && names.includes(remembered) ? remembered : resolveDefaultThinkingLevel(next));
    }

    function handleSelectConfirmMode(next: number): void {
        confirmModeService.setConfirmMode(next);
        if (!run.running || isNil(chatId)) {
            return;
        }
        agentService.handleConfirmMode(sessionId, {data: {aiChatId: chatId, confirmMode: next}});
    }

    function handleSelectThinkingLevel(next: string): void {
        if (!selected || !selected.id) {
            return;
        }
        aiProvideService.setThinkingLevel(selected.id, next);
        setThinkingLevel(next);
    }

    function dispatchSend(target: Editor, rawTitle: string): void {
        if (accountService.getAccount() === null) {
            setLoginAlert(t('account.login.fullFeaturesRequired'));
            return;
        }
        if (!aiShareConsentService.selectConsentDone()) {
            sharePendingRef.current = () => dispatchSend(target, rawTitle);
            setShareAsking(true);
            return;
        }
        if (isEmpty(send.resolveChatTitle(rawTitle))) {
            return;
        }
        if (isNotNil(revert.revertItemId)) {
            revert.askRevert();
            return;
        }
        if (isNotNil(queueApi.editingQueueId)) {
            queueApi.submitQueueEdit(target);
            return;
        }
        if (run.running) {
            queueApi.enqueueFromEditor(target);
            return;
        }
        if (isEmpty(chatId)) {
            send.insertAiChat(target);
            return;
        }
        send.handleChat(chatId, target);
    }

    function handleSend(): void {
        if (hints.pickActive() || !canSend() || !selected || isEmpty(selected.id)) {
            return;
        }
        const target = input.resolveEditor();
        if (!target) {
            return;
        }
        dispatchSend(target, serializeEditorMessage(target));
    }

    handleSendRef.current = handleSend;

    useImperativeHandle(ref, () => ({applyReference: input.applyReference, answerQuestion: send.answerQuestion, restoreDraft: input.restoreDraft, beginRevert: revert.beginRevert}));

    useReferenceDropZone({editor: input.editor, sessionId, panelRef, resolveEditor: input.resolveEditor, setDropOver});

    useKeepAwake(nativeMobile && (run.running || daemons.length > 0));

    useEffect(() => {
        input.loadCache(chatId);
    }, [chatId]);

    useEffect(() => {
        setConfirmMode(confirmModeService.getConfirmMode());
        return confirmModeService.addConfirmModeListener(setConfirmMode);
    }, [confirmModeService]);

    return (
        <InputFrame wrapRef={wrapRef} revert={revert}>
            <AlertDialog duration={4000} open={loginAlert != null} handleClose={() => setLoginAlert(null)} message={loginAlert} type="error"/>
            <ConfirmDialog
                open={shareAsking}
                title={t('agent.input.aiShareTitle')}
                message={t('agent.input.aiShareMessage')}
                onCancel={() => {
                    sharePendingRef.current = null;
                    setShareAsking(false);
                }}
                onConfirm={() => {
                    const run = sharePendingRef.current;
                    sharePendingRef.current = null;
                    setShareAsking(false);
                    aiShareConsentService.saveConsentDone();
                    if (run) {
                        run();
                    }
                }}
            >
                <Box sx={{mt: 1, fontSize: 13, lineHeight: 1.6, color: 'var(--tm-text-secondary)'}}>{t('agent.input.aiShareDetail')}</Box>
            </ConfirmDialog>
            {isNil(revert.revertItemId) ? <BackgroundTerminals chatId={chatId}/> : null}
            <InputHint trigger={hints.trigger} groups={hints.groups} active={hints.active} onPick={hints.handlePick} onPickServer={hints.handlePickServer} onExpand={hints.handleExpand} onToggle={hints.handleToggle}/>
            {isNil(revert.revertItemId) ? <QueuedMessages queue={queueApi.queue} editingId={queueApi.editingQueueId} onEdit={queueApi.beginQueueEdit} onDelete={queueApi.removeQueued} onSendNow={queueApi.forceSendQueued} onMove={queueApi.moveQueued}/> : null}
            {isNotNil(queueApi.editingQueueId) ? (
                <Box className="tm-agent-input-revert">
                    <Box className="tm-agent-input-revert-text">{t('agent.queue.editingHint')}</Box>
                    <ButtonBase disableRipple onClick={queueApi.cancelQueueEdit} aria-label={t('agent.queue.editCancelAria')} className="tm-agent-input-revert-close">
                        <Close sx={{fontSize: 14}}/>
                    </ButtonBase>
                </Box>
            ) : null}
            <Box ref={panelRef} className={dropOver ? 'tm-agent-input-panel is-drop-target' : 'tm-agent-input-panel'} data-tm-deck-input-panel="true" onClick={input.handlePanelClick}>
                <DropPaste editor={input.editor} remainingSlots={maxImageCount - images.length} panelRef={panelRef} onAddImages={handleAddImages} onDropOverChange={setDropOver}/>
                <Images images={images} onRemove={handleRemoveImage}/>
                <EditorContent editor={input.editor} className="tm-agent-input-editor-host" data-tm-deck-input-editor="true"/>
                <Toolbar
                    running={run.running}
                    canSend={canSend()}
                    confirmMode={confirmMode}
                    remainingSlots={maxImageCount - images.length}
                    chatId={chatId}
                    selectedId={resolveSelectedId()}
                    thinkingLevels={thinkingLevels}
                    thinkingLevel={thinkingLevel}
                    onSend={handleSend}
                    onStop={run.handleStop}
                    onAddImages={handleAddImages}
                    onSelectModel={handleSelectModel}
                    onSelectConfirmMode={handleSelectConfirmMode}
                    onSelectThinkingLevel={handleSelectThinkingLevel}
                    onInsertReference={input.insertReference}
                />
            </Box>
        </InputFrame>
    );
}
