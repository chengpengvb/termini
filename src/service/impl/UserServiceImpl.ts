import {createElement} from 'react';
import {inject, injectable} from 'inversify';
import type {UserService} from '../UserService.ts';
import {BaseListenerService} from './BaseListenerService.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {BAR_TOOL_SLOT} from '@/common/components/tab/core/types.ts';
import {UserAvatarBadge} from '@/page/account/user/UserAvatarBadge.tsx';
import type {TabManagerService} from '@/common/components/tab/service/TabManagerService.ts';
import {TabManagerServiceImpl} from '@/common/components/tab/service/impl/TabManagerServiceImpl.ts';

@injectable()
export class UserServiceImpl extends BaseListenerService implements UserService {
    private popoverOpen = false;
    private loginOpen = false;
    private static readonly LISTENER_KEY = 'user-popover';
    private static readonly LOGIN_LISTENER_KEY = 'user-login';

    constructor(@inject(TabManagerServiceImpl) private readonly tabManagerService: TabManagerService) {
        super();
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    override async start(): Promise<void> {
        void this.tabManagerService.waitReady().then(() => {
            this.tabManagerService.addBarTool({
                tabBar: {
                    slot: BAR_TOOL_SLOT.RIGHT,
                    icon: createElement(UserAvatarBadge),
                },
            });
        });
    }

    isPopoverOpen(): boolean {
        return this.popoverOpen;
    }

    setPopoverOpen(open: boolean): void {
        this.popoverOpen = open;
        this.notifyListeners<boolean>(UserServiceImpl.LISTENER_KEY, open);
    }

    togglePopover(): void {
        this.setPopoverOpen(!this.popoverOpen);
    }

    addPopoverListener(cb: (open: boolean) => void): () => void {
        return this.addListener<boolean>(UserServiceImpl.LISTENER_KEY, cb);
    }

    isLoginOpen(): boolean {
        return this.loginOpen;
    }

    setLoginOpen(open: boolean): void {
        this.loginOpen = open;
        this.notifyListeners<boolean>(UserServiceImpl.LOGIN_LISTENER_KEY, open);
    }

    addLoginListener(cb: (open: boolean) => void): () => void {
        return this.addListener<boolean>(UserServiceImpl.LOGIN_LISTENER_KEY, cb);
    }
}
