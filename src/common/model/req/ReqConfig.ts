export interface SaveNotifyReq {
    enabled: boolean;
    webhookKind: number;
    trayEnabled: boolean;
    email: string | null;
    webhookEnabled: boolean;
    webhookUrl: string | null;
    webhookExtra: string | null;
}
