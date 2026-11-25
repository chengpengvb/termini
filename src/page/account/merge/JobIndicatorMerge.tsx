import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'arg', labelKey: 'account.merge.fields.arg'},
    {key: 'protocol', labelKey: 'account.merge.fields.protocol'},
    {key: 'name', labelKey: 'account.merge.fields.indicatorName'},
    {key: 'enabled', labelKey: 'account.merge.fields.collectEnabled'},
    {key: 'parser_type', labelKey: 'account.merge.fields.parserType'},
    {key: 'parse_expr', labelKey: 'account.merge.fields.parseExpr'},
    {key: 'display_unit', labelKey: 'account.merge.fields.displayUnit'},
    {key: 'alert_enabled', labelKey: 'account.merge.fields.alertEnabled'},
    {key: 'alert_rule_op', labelKey: 'account.merge.fields.alertRuleOp'},
    {key: 'alert_threshold', labelKey: 'account.merge.fields.alertThreshold'},
];

export function JobIndicatorMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
