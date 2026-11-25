import type {Extension} from '@codemirror/state';
import {isNil} from '@/common/util/StringUtil.ts';
import {languages} from '@codemirror/language-data';
import {LanguageDescription} from '@codemirror/language';

export async function resolveLanguageExtension(fileName: string): Promise<Extension | null> {
    const description = LanguageDescription.matchFilename(languages, fileName);
    if (isNil(description)) {
        return null;
    }
    return description.load();
}
