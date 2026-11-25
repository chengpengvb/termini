export interface ShellDescr {
    exe?: string;
    args: string[];
    shell_id: string;
    env: [string, string][];
    supports_osc633: boolean;
}