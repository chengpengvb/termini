import {TerminalService} from "@/service/TerminalService.ts";
import type {ConfigSerial} from "@/common/model/ConfigSerial.ts";

export interface SerialService extends TerminalService {
    ports(): Promise<string[]>;

    config(portName: string): Promise<ConfigSerial | null>;

    saveConfig(config: ConfigSerial): Promise<ConfigSerial>;

    addPortsChangeListener(cb: (ports: string[]) => void): () => void;
}
