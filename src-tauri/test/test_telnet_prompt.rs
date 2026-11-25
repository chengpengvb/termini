use crate::model::config_telnet::ConfigTelnet;
use crate::module::telnet::prompt::prompt_text;
use crate::module::telnet::prompt::prompt_pattern::TelnetPromptPattern;

fn pattern() -> TelnetPromptPattern {
    TelnetPromptPattern::from_config(&ConfigTelnet::default())
}

#[test]
fn detect_username_prompt() {
    let pattern = pattern();
    assert!(pattern.is_username_prompt("\r\nLogin authentication\r\n\r\nUsername:"), "华为 telnet 用户名提示应被识别");
    assert!(pattern.is_username_prompt("\r\nUser Access Verification\r\n\r\nUsername: "), "思科用户名提示应被识别");
    assert!(pattern.is_username_prompt("****** banner ******\r\n\r\nlogin: "), "H3C login 提示应被识别");
    assert!(!pattern.is_username_prompt("Last login: Mon Jul 27 09:12:03 2026\r\nsome banner text"), "横幅里的 Last login 不应被当成用户名提示");
}

#[test]
fn detect_password_prompt() {
    let pattern = pattern();
    assert!(pattern.is_password_prompt("\r\nPassword:"), "密码提示应被识别");
    assert!(pattern.is_password_prompt("\r\npassword: "), "小写密码提示应被识别");
    assert!(!pattern.is_password_prompt("Password required, but none set\r\nUsername:"), "末行是用户名提示时不应判为密码提示");
}

#[test]
fn detect_device_prompt() {
    let pattern = pattern();
    assert!(pattern.is_device_prompt("\r\n<HUAWEI>"), "华为用户视图提示符应被识别");
    assert!(pattern.is_device_prompt("\r\n[HUAWEI-GigabitEthernet0/0/1]"), "华为系统视图提示符应被识别");
    assert!(pattern.is_device_prompt("\r\n<H3C>"), "H3C 提示符应被识别");
    assert!(pattern.is_device_prompt("\r\nSwitch#"), "思科特权模式提示符应被识别");
    assert!(pattern.is_device_prompt("\r\nSwitch(config)# "), "思科配置模式提示符应被识别");
    assert!(pattern.is_device_prompt("\r\nSwitch>"), "思科用户模式提示符应被识别");
}

#[test]
fn device_prompt_ignores_hash_lines_inside_output() {
    let pattern = pattern();
    //Comware 的配置输出里满是裸 `#` 分隔行，只看末行才不会把它们误判成命令已结束
    let output = "#\r\n version 7.1.070\r\n#\r\n sysname H3C\r\n#\r\n interface Vlan-interface1";
    assert!(!pattern.is_device_prompt(output), "配置正文中的 # 分隔行不应被判为提示符");
}

#[test]
fn detect_more_prompt_variants() {
    let pattern = pattern();
    for text in ["--More--", "  ---- More ----  ", "--More(85%)--", "---(more 62%)---", "-- MORE --"] {
        assert!(pattern.find_more_prompt(text).is_some(), "分页提示应被识别: {}", text);
    }
    assert!(pattern.find_more_prompt(" sysname H3C").is_none(), "普通输出不应被判为分页提示");
}

#[test]
fn command_end_waits_for_real_prompt() {
    let pattern = pattern();
    let streaming = "display current-configuration\n#\n version 7.1.070\n#\n sysname H3C\n#\n";
    assert!(!pattern.is_command_end("<H3C>", streaming), "输出正文里出现主机名不应被判为命令结束");
    assert!(pattern.is_command_end("<H3C>", &format!("{}return\n<H3C>", streaming)), "提示符复现时应判为命令结束");
}

#[test]
fn command_end_tolerates_mode_switch() {
    let pattern = pattern();
    assert!(pattern.is_command_end("<HUAWEI>", "system-view\nEnter system view, return user view with return command.\n[HUAWEI]"), "切换视图后提示符变形，仍应能收口");
    assert!(pattern.is_command_end("<HUAWEI>", "interface GigabitEthernet0/0/1\n[HUAWEI-GigabitEthernet0/0/1]"), "进接口视图后提示符追加后缀，仍应能收口");
    assert!(pattern.is_command_end("Switch>", "enable\nSwitch#"), "进特权模式后终止符变化，仍应能收口");
}

#[test]
fn command_end_rejects_interactive_question() {
    let pattern = pattern();
    assert!(!pattern.is_command_end("<HUAWEI>", "delete flash:/x.cfg\nWarning: The file will be deleted. Continue? [Y/N]"), "设备在等确认时不应判为命令结束");
}

#[test]
fn extract_base_prompt_strips_decorations() {
    assert_eq!(prompt_text::extract_base_prompt("<HUAWEI>"), "HUAWEI");
    assert_eq!(prompt_text::extract_base_prompt("[Switch-Vlan-interface10]"), "Switch-Vlan-interface10");
    assert_eq!(prompt_text::extract_base_prompt("Switch(config)#"), "Switch(config)");
    assert_eq!(prompt_text::extract_base_prompt("  Switch>  "), "Switch");
}

#[test]
fn strip_ansi_removes_huawei_cursor_left_with_space() {
    //华为在光标左移前多插一个空格，不连空格一起删提示符尾部就会多出空白导致失配
    let input = "<HUAWEI> \u{1b}[1D";
    assert_eq!(prompt_text::strip_ansi_escape_codes(input), "<HUAWEI>");
    assert_eq!(prompt_text::strip_ansi_escape_codes("\u{1b}[2J\u{1b}[32mok\u{1b}[0m"), "ok");
}

#[test]
fn normalize_linefeeds_collapses_carriage_returns() {
    assert_eq!(prompt_text::normalize_linefeeds("a\r\r\nb\r\nc\rd"), "a\nb\nc\nd");
}

#[test]
fn sanitize_removes_command_echo_and_trailing_prompt() {
    let raw = "display version\r\nHuawei Versatile Routing Platform Software\r\nVRP (R) software, Version 5.170\r\n<HUAWEI>";
    let text = prompt_text::normalize_linefeeds(&prompt_text::strip_ansi_escape_codes(raw));
    let text = prompt_text::strip_command_echo("display version", &text);
    let text = prompt_text::strip_trailing_prompt("<HUAWEI>", &text);
    let output = prompt_text::trim_line_ends(&text);
    assert_eq!(output.trim_end(), "Huawei Versatile Routing Platform Software\nVRP (R) software, Version 5.170");
}

#[test]
fn trim_line_ends_removes_paging_erase_spaces() {
    //擦除分页标记留下的成排空格在剥掉 ANSI 后会变成整行空白
    assert_eq!(prompt_text::trim_line_ends("line one\n          \nline two"), "line one\n\nline two");
}
