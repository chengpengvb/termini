import {translate} from '@/common/i18n/i18n.ts';

/** 与后端 `CollectProtocol` 一一对应 */
export const JOB_PROTOCOL = {
    SSH: 1,
    TELNET: 2,
    ICMP: 3,
    SNMP: 4,
    HTTP: 5,
    TCP: 6,
    TLS_CERT: 7,
} as const;

/** 与后端 `ParserType` 一一对应 */
export const JOB_PARSER_TYPE = {
    EXIT_CODE: 1,
    REGEX: 2,
    JSON_PATH: 3,
} as const;

/** 与后端 `JobIndicatorAlertRuleOp` 一一对应 */
export const JOB_ALERT_RULE_OP = {
    GT: 1,
    GTE: 2,
    LT: 3,
    LTE: 4,
    EQ: 5,
} as const;

/** 与后端 `JobIndicatorDisplayUnit` 一一对应，只影响前端怎么显示，存和比的都是原值 */
export const JOB_DISPLAY_UNIT = {
    NONE: 0,
    BYTES: 1,
    BITRATE: 2,
    MILLIS: 3,
    SECONDS: 4,
    COUNT: 5,
} as const;

/** 与后端 `JobIndicatorAlertAiOn` 一一对应 */
export const JOB_ALERT_AI_ON = {
    OFF: 0,
    ANALYZE: 1,
    REPAIR_CONFIRM: 2,
} as const;

export const INDICATOR_ARG_TYPE = {
    SHELL: 1,
    NUMBER: 2,
    SNMP: 3,
} as const;

export const JOB_INTERVAL_SECS_MIN = 60;
export const ICMP_COUNT_DEFAULT = '4';
export const TLS_CERT_PORT_DEFAULT = '443';

export interface IndicatorArgTemplate {
    id: string;
    protocol: number;
    argType: number;
    labelI18n: string;
}

export interface IndicatorArg {
    unit: string;
    value: string;
    template: IndicatorArgTemplate;
}

export interface JobIndicator {
    id: string;
    name: string;
    sort: number;
    enabled: boolean;
    protocol: number;
    parse_expr: string;
    alert_ai_on: number;
    is_default: boolean;
    parser_type: number;
    display_unit: number;
    alert_rule_op: number;
    alert_enabled: boolean;
    transform_expr: string;
    alert_threshold: string;
    alert_sensitivity: number;
    range_min: number | null;
    range_max: number | null;
    arg: IndicatorArg | null;
    alert_ai_repair_message?: string | null;
}

export function buildIndicatorArg(template: IndicatorArgTemplate, value: string, unit: string): IndicatorArg {
    return {unit, value, template};
}

export function protocolLabel(protocol: number): string {
    if (protocol === JOB_PROTOCOL.SSH) {
        return 'SSH';
    }
    if (protocol === JOB_PROTOCOL.TELNET) {
        return 'TELNET';
    }
    if (protocol === JOB_PROTOCOL.SNMP) {
        return 'SNMP';
    }
    if (protocol === JOB_PROTOCOL.HTTP) {
        return 'HTTP';
    }
    if (protocol === JOB_PROTOCOL.TCP) {
        return 'TCP';
    }
    if (protocol === JOB_PROTOCOL.TLS_CERT) {
        return translate('home.job.protocolTlsCert');
    }
    return 'ICMP';
}

export function parserTypeLabel(parserType: number): string {
    const known = parserType === JOB_PARSER_TYPE.EXIT_CODE || parserType === JOB_PARSER_TYPE.JSON_PATH;
    return translate(`home.job.alert.parserType.${known ? parserType : JOB_PARSER_TYPE.REGEX}`);
}

export function ruleOpLabel(op: number): string {
    if (op === JOB_ALERT_RULE_OP.GT) {
        return '>';
    }
    if (op === JOB_ALERT_RULE_OP.GTE) {
        return '>=';
    }
    if (op === JOB_ALERT_RULE_OP.LT) {
        return '<';
    }
    if (op === JOB_ALERT_RULE_OP.LTE) {
        return '<=';
    }
    return '=';
}
