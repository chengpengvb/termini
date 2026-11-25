pub mod parser_regex;
pub mod parser_exit_code;
pub mod parser_json_path;
pub mod parser_transform;

use crate::collect::dto::CollectContext;
use crate::model::job_indicator::JobIndicatorParserType;
use crate::model::job_indicator_value::JobIndicatorValue;

pub fn parse(parser_type: i16, parse_expr: &str, exit_code: Option<i32>, output: &str) -> anyhow::Result<Option<String>> {
    let Some(parser_type) = JobIndicatorParserType::from_i16(parser_type) else {
        anyhow::bail!("unknown parser type ({})", parser_type);
    };
    match parser_type {
        JobIndicatorParserType::ExitCode => Ok(parser_exit_code::parse(exit_code)),
        JobIndicatorParserType::Regex => parser_regex::parse(parse_expr, output),
        JobIndicatorParserType::JsonPath => parser_json_path::parse(parse_expr, output),
    }
}

pub fn build_value(ctx: &CollectContext, exit_code: Option<i32>, output: &str) -> JobIndicatorValue {
    let parsed = match parse(ctx.parser_type(), &ctx.parse_expr(), exit_code, output) {
        Ok(Some(value)) => value,
        Ok(None) => return ctx.new_value(None, false, Some("command succeeded but the parse expression matched nothing; check the expression against the command output".to_string())),
        Err(error) => return ctx.new_value(None, false, Some(format!("parse expression is invalid, fix it and retry: {}", error))),
    };
    match parser_transform::transform(&ctx.transform_expr(), &parsed) {
        Ok(value) => ctx.new_value(Some(value), true, None),
        Err(error) => ctx.new_value(None, false, Some(format!("value transform script failed, fix it and retry: {}", error))),
    }
}