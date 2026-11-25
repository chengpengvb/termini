export const AI_SKILL_DESCRIPTION_MAX_LENGTH = 255;

export interface AiSkill {
    sort: number;
    id: string | null;
    name: string | null;
    content: string | null;
    description: string | null;
}
