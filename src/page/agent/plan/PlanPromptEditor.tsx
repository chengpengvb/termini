import {Box} from '@mui/material';
import type {Editor} from '@tiptap/react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {EditorContent, useEditor} from '@tiptap/react';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {PlanAddMenu} from '@/page/agent/plan/PlanAddMenu.tsx';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {DropPaste} from '@/page/agent/main/input/components/drop/DropPaste.tsx';
import {createReferenceEditorOptions, insertEditorReference, serializeEditorMessage, setEditorMessage} from '@/page/common/util/reference/editor.ts';

interface PlanPromptEditorProps {
    value: string;
    onChange: (text: string) => void;
}

export function PlanPromptEditor(props: PlanPromptEditorProps) {
    const {t} = useI18n();
    const {value, onChange} = props;
    const hydratedRef = useRef<boolean>(false);
    const editorRef = useRef<Editor | null>(null);
    const [dropOver, setDropOver] = useState<boolean>(false);
    const panelRef = useRef<HTMLDivElement | null>(null);

    const editor = useEditor(createReferenceEditorOptions({
        placeholder: t('agent.plan.promptPlaceholder'),
        onSubmit: () => {
            editorRef.current?.commands.setHardBreak();
        },
        onUpdate: target => {
            onChange(serializeEditorMessage(target));
        },
    }));
    editorRef.current = editor;

    const handleInsertReference = (ref: ChatReference): void => {
        if (!editor) {
            return;
        }
        insertEditorReference(editor, ref, null);
    };

    useEffect(() => {
        if (!editor || hydratedRef.current) {
            return;
        }
        hydratedRef.current = true;
        if (isNotEmpty(value)) {
            setEditorMessage(editor, value);
        }
    }, [editor, value]);

    return (
        <Box ref={panelRef} className={dropOver ? 'tm-agent-plan-editor is-drop-target' : 'tm-agent-plan-editor'}>
            <DropPaste editor={editor} remainingSlots={0} panelRef={panelRef} onAddImages={() => undefined} onDropOverChange={setDropOver}/>
            <EditorContent editor={editor} className="tm-agent-input-editor-host tm-agent-plan-editor-host"/>
            <Box className="tm-agent-plan-editor-toolbar">
                <PlanAddMenu onInsertReference={handleInsertReference}/>
            </Box>
        </Box>
    );
}
