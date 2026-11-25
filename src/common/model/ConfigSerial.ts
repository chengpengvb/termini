export interface ConfigSerial {
    id?: string;
    port_name: string;
    baud_rate: number;
    data_bits: number;
    stop_bits: number;
    parity: string;
    flow_control: string;
}
