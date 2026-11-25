export interface AiChatItem {
    id: string;
    type: number;
    context: any;
    chat_id: string;
    timestamp: number;
    pid?: string | null;
}
