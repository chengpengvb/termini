import type { BaseService } from './BaseService.ts';

export interface UserService extends BaseService {
    togglePopover(): void;
    isLoginOpen(): boolean;
    isPopoverOpen(): boolean;
    setLoginOpen(open: boolean): void;
    setPopoverOpen(open: boolean): void;
    addLoginListener(cb: (open: boolean) => void): () => void;
    addPopoverListener(cb: (open: boolean) => void): () => void;
}
