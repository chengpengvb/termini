import type { PointerEvent } from 'react';
import { inject, injectable } from 'inversify';
import { API_BASE_URL } from '@/runtime-config.ts';
import { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { BaseServiceImpl } from '@/service/impl/BaseServiceImpl.ts';
import { SftpServiceImpl } from "@/service/impl/SftpServiceImpl.ts";
import type { SftpDragService } from '@/service/SftpDragService.ts';
import { SftpTransferEvent } from "@/common/model/req/event/WsSftp.ts";
import { BaseApiServiceImpl } from '@/service/impl/BaseApiServiceImpl.ts';
import { ReqSftpStartDrag, ReqSftpStartRemoteDrag } from "@/common/model/req/ReqSftp.ts";
import { SFTP_PAGE_DRAG_THRESHOLD, SFTP_PAGE_PANEL } from '@/page/common/dto/sftp/SftpHeadConst.ts';
import type { SftpDragHandle, SftpDragOptions, SftpDragSession } from '@/page/common/dto/sftp/SftpDrag.ts';

interface SftpDragRuntime {
    session: SftpDragSession | null;
}

@injectable()
export class SftpDragServiceImpl extends BaseServiceImpl implements SftpDragService {
    private readonly runtimes = new Map<string, SftpDragRuntime>();

    constructor(
        @inject(SftpServiceImpl) private sftpService: SftpServiceImpl,
    ) {
        super();
    }

    createHandle(controllerId: string, options: SftpDragOptions): SftpDragHandle {
        return {
            handlePointerUp: event => this.handlePointerUp(controllerId, options, event),
            handlePointerMove: event => this.handlePointerMove(controllerId, options, event),
            handlePointerDown: event => this.handlePointerDown(controllerId, options, event),
        };
    }

    disposeHandle(controllerId: string): void {
        this.runtimes.delete(controllerId);
    }

    private handlePointerDown(controllerId: string, options: SftpDragOptions, event: PointerEvent<HTMLDivElement>): void {
        if (event.button !== 0) {
            return;
        }
        const panel = this.resolvePanel(options, event.clientX, event.clientY);
        if (!panel) {
            return;
        }
        const list = this.resolveHandle(options, panel)?.getRootElement()?.querySelector<HTMLElement>('.tm-sftp-entries');
        if (!list || !this.isPointInsideElement(event.clientX, event.clientY, list)) {
            return;
        }
        const runtime = this.resolveRuntime(controllerId);
        runtime.session = {
            active: false,
            startPanel: panel,
            osDragStarted: false,
            startX: event.clientX,
            startY: event.clientY,
            fromSelection: false,
            inTransferMode: false,
            pointerId: event.pointerId,
            append: this.resolveAppend(event),
        };
    }

    private hidePreview(options: SftpDragOptions): void {
        const preview = options.previewRef.current;
        if (!preview) {
            return;
        }
        preview.hide();
    }

    private movePreview(options: SftpDragOptions, clientX: number, clientY: number): void {
        options.previewRef.current?.move(clientX, clientY);
    }

    private showPreview(options: SftpDragOptions, clientX: number, clientY: number, count: number, targetPath: string | undefined): void {
        options.previewRef.current?.show(clientX, clientY, count, targetPath);
    }

    private resolvePanelCurrentPath(options: SftpDragOptions, panel: number): string | undefined {
        return this.resolveHandle(options, panel)?.getCurrentPath();
    }

    private resolveSelectedNamesCount(options: SftpDragOptions, panel: number): number {
        return this.resolveHandle(options, panel)?.getSelectedNames().length || 0;
    }

    private activateDragSessionIfNeeded(options: SftpDragOptions, session: SftpDragSession, event: PointerEvent<HTMLDivElement>): void {
        if (session.active) {
            return;
        }
        const dx = Math.abs(event.clientX - session.startX);
        const dy = Math.abs(event.clientY - session.startY);
        if (dx < SFTP_PAGE_DRAG_THRESHOLD && dy < SFTP_PAGE_DRAG_THRESHOLD) {
            return;
        }
        session.active = true;
        try {
            event.currentTarget.setPointerCapture(event.pointerId);
        } catch {
        }
        this.beginSourceDragSelect(options, session);
    }

    private isPointerOutsideWindow(clientX: number, clientY: number): boolean {
        if (clientX < 0 || clientY < 0) {
            return true;
        }
        return clientX > window.innerWidth || clientY > window.innerHeight;
    }

    private handlePointerMoveOsDragOut(controllerId: string, options: SftpDragOptions, session: SftpDragSession, event: PointerEvent<HTMLDivElement>): void {
        session.osDragStarted = true;
        const sourceHandle = this.resolveHandle(options, session.startPanel);
        if (!session.fromSelection && sourceHandle) {
            sourceHandle.endDragSelect(event.clientX, event.clientY);
        }
        this.hidePreview(options);
        this.hideAllDragCovers(options);
        try {
            event.currentTarget.releasePointerCapture(event.pointerId);
        } catch {
        }
        this.disposeHandle(controllerId);
        this.triggerOsDragOut(options, session, sourceHandle, event);
    }

    private handlePointerMoveIdle(options: SftpDragOptions, session: SftpDragSession, currentPanel: number | null, event: PointerEvent<HTMLDivElement>): void {
        if (currentPanel !== null && currentPanel !== session.startPanel) {
            session.inTransferMode = true;
            const sourceHandle = this.resolveHandle(options, session.startPanel);
            if (!session.fromSelection && sourceHandle) {
                sourceHandle.endDragSelect(event.clientX, event.clientY);
            }
            this.syncDragCover(options, session.startPanel, currentPanel);
            const targetPath = this.resolvePanelCurrentPath(options, currentPanel);
            const count = this.resolveSelectedNamesCount(options, session.startPanel);
            this.showPreview(options, event.clientX, event.clientY, count, targetPath);
            return;
        }
        if (session.fromSelection) {
            this.movePreview(options, event.clientX, event.clientY);
            return;
        }
        this.updateSourceDragSelect(options, session.startPanel, event.clientX, event.clientY);
    }

    private handlePointerMoveTransferring(options: SftpDragOptions, session: SftpDragSession, currentPanel: number | null, event: PointerEvent<HTMLDivElement>): void {
        if (currentPanel === session.startPanel) {
            session.inTransferMode = false;
            this.hideAllDragCovers(options);
            if (session.fromSelection) {
                const count = this.resolveSelectedNamesCount(options, session.startPanel);
                this.showPreview(options, event.clientX, event.clientY, count, undefined);
                return;
            }
            this.hidePreview(options);
            this.beginSourceDragSelect(options, session);
            this.updateSourceDragSelect(options, session.startPanel, event.clientX, event.clientY);
            return;
        }
        this.syncDragCover(options, session.startPanel, currentPanel);
        this.movePreview(options, event.clientX, event.clientY);
    }

    private handlePointerMove(controllerId: string, options: SftpDragOptions, event: PointerEvent<HTMLDivElement>): void {
        if (!this.runtimes.has(controllerId)) {
            return;
        }
        const runtime = this.resolveRuntime(controllerId);
        const session = runtime.session;
        if (!session || session.pointerId !== event.pointerId) {
            return;
        }
        this.activateDragSessionIfNeeded(options, session, event);
        if (!session.active || session.osDragStarted) {
            return;
        }
        if (this.isPointerOutsideWindow(event.clientX, event.clientY)) {
            this.handlePointerMoveOsDragOut(controllerId, options, session, event);
            return;
        }
        const currentPanel = this.resolvePanel(options, event.clientX, event.clientY);
        if (!session.inTransferMode) {
            this.handlePointerMoveIdle(options, session, currentPanel, event);
            return;
        }
        this.handlePointerMoveTransferring(options, session, currentPanel, event);
    }

    private handleTransfer(session: SftpDragSession, sourceHandle: SftpHandle | null, targetHandle: SftpHandle | null, options: SftpDragOptions, event: PointerEvent<HTMLDivElement>): void {
        const names = sourceHandle?.getSelectedNames();
        const sourcePath = sourceHandle?.getCurrentPath();
        const targetPath = targetHandle?.getCurrentPath();
        const sideSessionId = targetHandle?.getSessionId();
        const sessionId = sourceHandle?.getSessionId();
        const sourceClientTransferId = sourceHandle?.getClientTransferId();
        const targetClientTransferId = targetHandle?.getClientTransferId();
        const currentPanel = this.resolvePanel(options, event.clientX, event.clientY);
        if (!currentPanel || currentPanel === session.startPanel || !sourceHandle || !names || names.length === 0) {
            return;
        }
        if (!sourcePath || !targetPath || !sideSessionId || !sessionId) {
            return;
        }
        let payload: SftpTransferEvent = {
            data: {
                names: names,
                sourcePath: sourcePath,
                targetPath: targetPath,
                sideSessionId: sideSessionId,
                sourceClientTransferId: sourceClientTransferId,
                targetClientTransferId: targetClientTransferId
            }
        };
        this.sftpService.handleTransfer(sessionId, payload);
    }

    private handlePointerUp(controllerId: string, options: SftpDragOptions, event: PointerEvent<HTMLDivElement>): void {
        const runtime = this.resolveRuntime(controllerId);
        const session = runtime.session;
        if (!session || session.pointerId !== event.pointerId) {
            return;
        }
        try {
            event.currentTarget.releasePointerCapture(event.pointerId);
        } catch {
        }
        this.hideAllDragCovers(options);
        options.previewRef.current?.hide();
        if (!session.active) {
            this.disposeHandle(controllerId);
            return;
        }
        const sourceHandle = this.resolveHandle(options, session.startPanel);
        this.disposeHandle(controllerId);
        if (session.inTransferMode) {
            if (!session.fromSelection) {
                sourceHandle?.cancelDragSelect();
            }
            this.handleTransfer(session, sourceHandle, this.resolveSideHandle(options, session.startPanel), options, event);
            return;
        }
        if (!session.fromSelection) {
            sourceHandle?.endDragSelect(event.clientX, event.clientY);
        }
    }

    private triggerOsDragOut(_options: SftpDragOptions, _session: SftpDragSession, sourceHandle: SftpHandle | null, event: PointerEvent<HTMLDivElement>): void {
        if (!sourceHandle) {
            return;
        }
        const names = sourceHandle.getSelectedNames();
        if (names.length === 0) {
            return;
        }
        const currentPath = sourceHandle.getCurrentPath();
        if (!currentPath) {
            return;
        }
        if (sourceHandle.isLocal()) {
            const paths = names.map(name => `${currentPath}/${name}`);
            this.startLocalOsDrag(paths);
            return;
        }
        this.startRemoteOsDrag(sourceHandle, currentPath, names, event);
    }

    private startRemoteOsDrag(sourceHandle: SftpHandle, sourcePath: string, names: string[], event: PointerEvent<HTMLDivElement>): void {
        const sessionId = sourceHandle.getSessionId();
        const clientTransferId = sourceHandle.getClientTransferId();
        if (!clientTransferId || !sessionId) {
            return;
        }
        const body: ReqSftpStartRemoteDrag = {
            sessionId, clientTransferId, sourcePath, names, screenX: event.screenX, screenY: event.screenY,
        };
        fetch(`${API_BASE_URL}/sftp/drag/start_remote`, { method: 'POST', headers: { 'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders() }, body: JSON.stringify(body), }).catch(console.error);
    }

    private startLocalOsDrag(paths: string[]): void {
        const body: ReqSftpStartDrag = { paths };
        fetch(`${API_BASE_URL}/sftp/drag/start`, { method: 'POST', headers: { 'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders() }, body: JSON.stringify(body) }).catch(console.error);
    }

    private resolveRuntime(controllerId: string): SftpDragRuntime {
        const runtime = this.runtimes.get(controllerId);
        if (runtime) {
            return runtime;
        }
        const nextRuntime: SftpDragRuntime = { session: null };
        this.runtimes.set(controllerId, nextRuntime);
        return nextRuntime;
    }

    private resolvePanel(options: SftpDragOptions, clientX: number, clientY: number): number | null {
        const leftEl = options.leftRef.current?.getRootElement();
        if (leftEl && this.isPointInsideElement(clientX, clientY, leftEl)) {
            return SFTP_PAGE_PANEL.LEFT;
        }
        const rightEl = options.rightRef.current?.getRootElement();
        if (rightEl && this.isPointInsideElement(clientX, clientY, rightEl)) {
            return SFTP_PAGE_PANEL.RIGHT;
        }
        return null;
    }

    private isPointInsideElement(clientX: number, clientY: number, element: HTMLElement): boolean {
        const rect = element.getBoundingClientRect();
        if (clientX < rect.left || clientX > rect.right || clientY < rect.top) {
            return false;
        }
        return clientY <= rect.bottom;
    }

    private resolveHandle(options: SftpDragOptions, panel: number): SftpHandle | null {
        if (panel === SFTP_PAGE_PANEL.LEFT) {
            return options.leftRef.current;
        }
        return options.rightRef.current;
    }

    private resolveSideHandle(options: SftpDragOptions, panel: number): SftpHandle | null {
        if (panel === SFTP_PAGE_PANEL.LEFT) {
            return options.rightRef.current;
        }
        return options.leftRef.current;
    }

    private resolveAppend(event: PointerEvent<HTMLDivElement>): boolean {
        if (event.ctrlKey) {
            return true;
        }
        return event.metaKey;
    }

    private hideAllDragCovers(options: SftpDragOptions): void {
        options.leftRef.current?.hideDragCover();
        options.rightRef.current?.hideDragCover();
    }

    private syncDragCover(options: SftpDragOptions, startPanel: number, currentPanel: number | null): void {
        this.hideAllDragCovers(options);
        if (!currentPanel) {
            return;
        }
        if (currentPanel === startPanel) {
            return;
        }
        this.resolveHandle(options, currentPanel)?.showDragCover();
    }

    private beginSourceDragSelect(options: SftpDragOptions, session: SftpDragSession): void {
        this.resolveHandle(options, session.startPanel)?.beginDragSelect(session.startX, session.startY, session.append, ctx => {
            session.fromSelection = true;
            if (ctx.count === 0) {
                return;
            }
            options.previewRef.current?.show(ctx.clientX, ctx.clientY, ctx.count, undefined);
        },);
    }

    private updateSourceDragSelect(options: SftpDragOptions, startPanel: number, clientX: number, clientY: number): void {
        this.resolveHandle(options, startPanel)?.updateDragSelect(clientX, clientY);
    }
}
