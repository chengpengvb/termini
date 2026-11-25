export interface RespTree {
    id: string;
    title: string;
    type: number;
    data: string | null;
    children: RespTree[] | null;
}
