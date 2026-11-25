import {Box} from '@mui/material';
import {useEffect, useRef} from 'react';
import {MergeView} from '@codemirror/merge';
import {buildCodeTheme} from './codeTheme.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {EditorView, lineNumbers} from '@codemirror/view';
import {resolveLanguageExtension} from './codeLanguage.ts';
import {EditorState, type Extension} from '@codemirror/state';

interface CodeDiffProps {
    fileName: string;
    baselineText: string;
    currentText: string;
}

function buildDiffExtensions(languageExtension: Extension | null): Extension[] {
    const extensions = [
        lineNumbers(),
        EditorView.lineWrapping,
        EditorView.editable.of(false),
        EditorState.readOnly.of(true),
        ...buildCodeTheme(),
    ];
    if (isNil(languageExtension)) {
        return extensions;
    }
    return [...extensions, languageExtension];
}

export function CodeDiff(props: CodeDiffProps) {
    const {fileName, baselineText, currentText} = props;
    const hostRef = useRef<HTMLDivElement | null>(null);
    const mergeViewRef = useRef<MergeView | null>(null);

    useEffect(() => {
        const host = hostRef.current;
        if (isNil(host)) {
            return;
        }
        let cancelled = false;
        mergeViewRef.current?.destroy();
        mergeViewRef.current = null;
        host.innerHTML = '';
        void resolveLanguageExtension(fileName).then((languageExtension) => {
            if (cancelled) {
                return;
            }
            mergeViewRef.current = new MergeView({
                a: {doc: baselineText, extensions: buildDiffExtensions(languageExtension)},
                b: {doc: currentText, extensions: buildDiffExtensions(languageExtension)},
                parent: host,
                gutter: true,
                highlightChanges: true,
                collapseUnchanged: {margin: 3, minSize: 6},
            });
        });
        return () => {
            cancelled = true;
            mergeViewRef.current?.destroy();
            mergeViewRef.current = null;
        };
    }, [fileName, baselineText, currentText]);

    return (
        <Box ref={hostRef} className="tm-code-diff" data-tm-editor="true"/>
    );
}
