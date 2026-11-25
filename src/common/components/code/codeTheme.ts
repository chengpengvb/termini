import {tags} from '@lezer/highlight';
import {EditorView} from '@codemirror/view';
import type {Extension} from '@codemirror/state';
import {HighlightStyle, syntaxHighlighting} from '@codemirror/language';

const chalky = 'var(--tm-code-chalky)';
const coral = 'var(--tm-code-coral)';
const cyan = 'var(--tm-code-cyan)';
const ivory = 'var(--tm-code-ivory)';
const stone = 'var(--tm-code-stone)';
const malibu = 'var(--tm-code-malibu)';
const sage = 'var(--tm-code-sage)';
const whiskey = 'var(--tm-code-whiskey)';
const violet = 'var(--tm-code-violet)';

const codeHighlightStyle = HighlightStyle.define([
    {tag: tags.keyword, color: violet},
    {tag: [tags.name, tags.deleted, tags.character, tags.propertyName, tags.macroName], color: coral},
    {tag: [tags.function(tags.variableName), tags.labelName], color: malibu},
    {tag: [tags.color, tags.constant(tags.name), tags.standard(tags.name)], color: whiskey},
    {tag: [tags.definition(tags.name), tags.separator], color: ivory},
    {tag: [tags.typeName, tags.className, tags.number, tags.changed, tags.annotation, tags.modifier, tags.self, tags.namespace], color: chalky},
    {tag: [tags.operator, tags.operatorKeyword, tags.url, tags.escape, tags.regexp, tags.special(tags.string)], color: cyan},
    {tag: [tags.meta, tags.comment], color: stone},
    {tag: tags.strong, fontWeight: 'bold'},
    {tag: tags.emphasis, fontStyle: 'italic'},
    {tag: tags.strikethrough, textDecoration: 'line-through'},
    {tag: tags.link, color: stone, textDecoration: 'underline'},
    {tag: tags.heading, fontWeight: 'bold', color: coral},
    {tag: [tags.atom, tags.bool, tags.special(tags.variableName)], color: whiskey},
    {tag: [tags.processingInstruction, tags.string, tags.inserted], color: sage},
    {tag: tags.invalid, color: 'var(--tm-overlay-ink)'},
]);

export function buildCodeTheme(): Extension[] {
    return [
        EditorView.theme({
            '&': {height: '100%', fontSize: '12.5px', backgroundColor: 'transparent'},
            '&.cm-focused': {outline: 'none'},
            '.cm-content': {fontFamily: 'var(--tm-font-mono)', caretColor: 'var(--tm-text-primary)'},
            '.cm-tooltip': {backgroundColor: 'var(--tm-bg-elevated)', color: 'var(--tm-text-primary)', border: '1px solid var(--tm-border-subtle)'},
            '.cm-tooltip-autocomplete > ul > li[aria-selected]': {backgroundColor: 'var(--tm-bg-selected)', color: 'var(--tm-text-primary)'},
            '.cm-foldPlaceholder': {backgroundColor: 'var(--tm-bg-hover)', color: 'var(--tm-text-secondary)', borderColor: 'var(--tm-border-subtle)'},
            '.cm-gutters': {backgroundColor: 'transparent', border: 'none', color: 'var(--tm-text-tertiary)'},
            '.cm-cursor': {borderLeftColor: 'var(--tm-text-primary)'},
            '.cm-activeLine': {backgroundColor: 'color-mix(in srgb, var(--tm-overlay-ink) 4%, transparent)'},
            '.cm-activeLineGutter': {backgroundColor: 'transparent'},
            '.cm-selectionBackground': {backgroundColor: 'color-mix(in srgb, var(--tm-overlay-ink) 10%, transparent)'},
            '&.cm-focused .cm-selectionBackground': {backgroundColor: 'color-mix(in srgb, var(--tm-overlay-ink) 16%, transparent)'},
            '.cm-selectionMatch': {backgroundColor: 'rgba(229, 192, 123, 0.18)'},
            '.cm-searchMatch': {backgroundColor: 'rgba(97, 175, 239, 0.25)'},
            '.cm-searchMatch-selected': {backgroundColor: 'rgba(229, 192, 123, 0.35)'},
        }, {dark: true}),
        syntaxHighlighting(codeHighlightStyle),
    ];
}
