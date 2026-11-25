import type {BaseService} from './BaseService';

export interface TerminalThemeService extends BaseService {

    getAppThemeId(): string;

    setAppThemeId(themeId: string): void;

    getPaneThemeId(paneId: string): string;

    addChangeListener(listener: () => void): () => void;

    registerPaneTheme(paneId: string, themeId: string): void;

}
