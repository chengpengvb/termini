import type {JSONContent} from '@tiptap/core';

export interface ChatInputDraft {
    content: JSONContent | null;
    revert?: {
        itemId: string;
        originalContent: JSONContent | null;
    };
}
