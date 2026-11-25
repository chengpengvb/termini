import {ShellDescr} from "@/common/model/ShellDescr.ts";
import {TerminalService} from "@/service/TerminalService.ts";

export interface ShellService extends TerminalService {
    discover(): Promise<ShellDescr[]>;

}
