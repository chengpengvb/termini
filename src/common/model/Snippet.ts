import {isEmpty} from '../util/StringUtil';

export interface Snippet {
    id: string;
    name: string;
    command: string;
    sort: number;
    remark: string;
    updated_at: number;
}

const VARIABLE_PATTERN = /\{([^{}]+)\}/g;

export function createEmptySnippet(): Snippet {
    return {id: '', name: '', command: '', sort: 0, remark: '', updated_at: 0};
}

export function verifySnippet(value: Snippet): boolean {
    if (isEmpty(value.name)) {
        return false;
    }
    if (value.name.length > 64) {
        return false;
    }
    return !isEmpty(value.command);
}

export function matchSnippet(snippet: Snippet, keyword: string): boolean {
    if (snippet.name.toLowerCase().includes(keyword)) {
        return true;
    }
    return snippet.command.toLowerCase().includes(keyword);
}

export function snippetVariables(command: string): string[] {
    const names: string[] = [];
    for (const match of command.matchAll(VARIABLE_PATTERN)) {
        const name = match[1].trim();
        if (name && !names.includes(name)) {
            names.push(name);
        }
    }
    return names;
}

export function renderSnippetCommand(command: string, values: Record<string, string>): string {
    return command.replace(VARIABLE_PATTERN, (raw, name: string) => {
        const key = name.trim();
        if (key in values) {
            return values[key];
        }
        return raw;
    });
}
