import {Box} from '@mui/material';
import {useEffect, useRef} from 'react';
import {buildCodeTheme} from './codeTheme.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {Compartment, EditorState} from '@codemirror/state';
import {resolveLanguageExtension} from './codeLanguage.ts';
import {search, searchKeymap, highlightSelectionMatches} from '@codemirror/search';
import {defaultKeymap, history, historyKeymap, indentWithTab} from '@codemirror/commands';
import {drawSelection, EditorView, highlightActiveLine, keymap, lineNumbers} from '@codemirror/view';

interface CodeEditorProps {
    fileName: string;
    value: string;
    onDocChange: (doc: string) => void;
    onSave: () => void;
}

export function CodeEditor(props: CodeEditorProps) {
    const {fileName, value, onDocChange, onSave} = props;
    const hostRef = useRef<HTMLDivElement | null>(null);
    const onSaveRef = useRef(onSave);
    const onDocChangeRef = useRef(onDocChange);
    onSaveRef.current = onSave;
    onDocChangeRef.current = onDocChange;

    useEffect(() => {
        const host = hostRef.current;
        if (isNil(host)) {
            return;
        }
        const language = new Compartment();
        const view = new EditorView({
            state: EditorState.create({
                doc: value,
                extensions: [
                    lineNumbers(),
                    history(),
                    drawSelection(),
                    highlightActiveLine(),
                    highlightSelectionMatches(),
                    search({top: true}),
                    EditorView.lineWrapping,
                    keymap.of([
                        {key: 'Mod-s', preventDefault: true, run: () => {
                            onSaveRef.current();
                            return true;
                        }},
                        ...defaultKeymap,
                        ...historyKeymap,
                        ...searchKeymap,
                        indentWithTab,
                    ]),
                    language.of([]),
                    ...buildCodeTheme(),
                    EditorView.updateListener.of((update) => {
                        if (!update.docChanged) {
                            return;
                        }
                        onDocChangeRef.current(update.state.doc.toString());
                    }),
                ],
            }),
            parent: host,
        });
        let cancelled = false;
        void resolveLanguageExtension(fileName).then((extension) => {
            if (cancelled || isNil(extension)) {
                return;
            }
            view.dispatch({effects: language.reconfigure(extension)});
        });
        return () => {
            cancelled = true;
            view.destroy();
        };
    }, [fileName]);

    return (
        <Box ref={hostRef} className="tm-code-editor" data-tm-editor="true"/>
    );
}
