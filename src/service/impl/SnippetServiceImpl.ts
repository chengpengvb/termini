import {inject, injectable} from 'inversify';
import type {SnippetService} from '../SnippetService';
import type {Snippet} from '@/common/model/Snippet.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class SnippetServiceImpl extends BaseApiServiceImpl<Snippet> implements SnippetService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.SNIPPET;
    }

    protected getPrefix(): string {
        return '/snippet';
    }
}
