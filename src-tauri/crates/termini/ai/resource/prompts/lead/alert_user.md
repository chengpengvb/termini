[告警现场]
设备：{{device_name}}（device_id={{device_id}}，host={{device_host}}，系统={{device_os}}）
指标：{{indicator_name}}，单位 {{indicator_unit}}
采集方式：{{indicator_protocol}}，取值命令 {{indicator_command}}
解析方式：{{indicator_parser}}，表达式 {{indicator_parse_expr}}
触发条件：实测 {{alert_value}} {{alert_op}} 阈值 {{alert_threshold}}
触发时间：{{alert_at}}
采集是否成功：{{alert_success}}
采集报错：{{alert_error}}

[设备档案]
{{device_profile}}

[最近采集样本]
{{recent_samples}}

请按系统提示的四段结论格式，查清这条告警的根因并给出结论。
