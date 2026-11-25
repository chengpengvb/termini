const ALLOWED_COMMANDS: &[&str] = &[
    "rg",
    "grep",
    "egrep",
    "fgrep",
    "find",
    "fd",
    "ls",
    "dir",
    "tree",
    "wc",
    "sort",
    "uniq",
    "cut",
    "tr",
    "head",
    "tail",
    "cat",
    "file",
    "stat",
    "du",
    "df",
    "pwd",
    "which",
    "where",
    "whereis",
    "type",
    "echo",
    "printf",
    "basename",
    "dirname",
    "realpath",
    "readlink",
    "column",
    "nl",
    "diff",
    "cmp",
    "comm",
    "strings",
    "xxd",
    "hexdump",
    "od",
    "md5sum",
    "sha1sum",
    "sha256sum",
    // PowerShell 只读 cmdlet 及别名
    "get-childitem",
    "gci",
    "get-content",
    "gc",
    "get-item",
    "gi",
    "select-string",
    "sls",
    "measure-object",
    "measure",
    "select-object",
    "select",
    "where-object",
    "?",
    "foreach-object",
    "foreach",
    "%",
    "sort-object",
    "group-object",
    "group",
    "format-table",
    "ft",
    "format-list",
    "fl",
    "format-wide",
    "fw",
    "out-string",
    "get-location",
    "gl",
    "get-date",
    "test-path",
    "resolve-path",
    "rvpa",
    "split-path",
    "join-path",
    "write-output",
    "write-host",
    "get-command",
    "gcm",
    "get-filehash",
    "compare-object",
    "compare",
    "convertfrom-json",
    "convertto-json",
    "convertfrom-csv",
    "convertto-csv",
    "get-process",
    "gps",
    "get-service",
    "gsv",
    "get-computerinfo",
    "get-host",
    "get-unique",
    "gu",
    "out-host",
    "oh",
    // 版本管理 / 构建工具的只读子命令另有细则
    "git",
    "mvn",
    "gradle",
    "gradlew",
    "cargo",
    "npm",
    "pnpm",
    "yarn",
    "node",
    "python",
    "python3",
    "java",
    "javac",
    "go",
    "dotnet",
    "rustc",
];

const FORBIDDEN_ANYWHERE: &[&str] = &[
    "rm",
    "rmdir",
    "del",
    "erase",
    "rd",
    "mv",
    "move",
    "cp",
    "copy",
    "mkdir",
    "md",
    "touch",
    "ln",
    "chmod",
    "chown",
    "chgrp",
    "truncate",
    "shred",
    "dd",
    "tee",
    "remove-item",
    "ri",
    "rmi",
    "move-item",
    "mi",
    "copy-item",
    "cpi",
    "new-item",
    "ni",
    "set-content",
    "sc",
    "add-content",
    "ac",
    "clear-content",
    "clc",
    "out-file",
    "rename-item",
    "rni",
    "ren",
    "set-item",
    "si",
    "clear-item",
    "cli",
    "invoke-item",
    "ii",
    "invoke-expression",
    "iex",
    "invoke-command",
    "icm",
    "start-process",
    "saps",
    "start",
    "stop-process",
    "spps",
    "kill",
    "pkill",
    "killall",
    "taskkill",
    "stop-service",
    "restart-service",
    "set-location",
    "cd",
    "sl",
    "chdir",
    "push-location",
    "pushd",
    "set-executionpolicy",
    "remove-module",
    "install-module",
    "install-package",
    "register-scheduledtask",
    "new-service",
    "set-itemproperty",
    "sp",
    "remove-itemproperty",
    "rp",
    "new-itemproperty",
    "clear-host",
    "cls",
    "clear",
    "sudo",
    "su",
    "doas",
    "runas",
    "pkexec",
    "osascript",
    "cmd",
    "cmd.exe",
    "powershell",
    "pwsh",
    "sh",
    "bash",
    "zsh",
    "fish",
    "eval",
    "exec",
    "source",
    "xargs",
    "env",
    "nohup",
    "setsid",
    "screen",
    "tmux",
    "curl",
    "wget",
    "invoke-webrequest",
    "iwr",
    "invoke-restmethod",
    "irm",
    "ssh",
    "scp",
    "sftp",
    "rsync",
    "ftp",
    "telnet",
    "nc",
    "ncat",
    "netcat",
    "ping",
    "nmap",
    "apt",
    "apt-get",
    "yum",
    "dnf",
    "brew",
    "choco",
    "winget",
    "scoop",
    "pip",
    "pip3",
    "gem",
    "docker",
    "podman",
    "kubectl",
    "helm",
    "systemctl",
    "service",
    "launchctl",
    "reg",
    "regedit",
    "sc.exe",
    "net",
    "netsh",
    "diskpart",
    "format",
    "mkfs",
    "mount",
    "umount",
    "shutdown",
    "reboot",
    "poweroff",
    "halt",
];

const GIT_READ_SUBCOMMANDS: &[&str] = &[
    "log",
    "show",
    "diff",
    "status",
    "blame",
    "grep",
    "ls-files",
    "ls-tree",
    "rev-parse",
    "rev-list",
    "shortlog",
    "describe",
    "cat-file",
    "count-objects",
    "for-each-ref",
    "name-rev",
    "reflog",
    "help",
    "version",
    "branch",
    "tag",
    "remote",
    "config",
    "stash",
    "submodule",
    "worktree",
];
const GIT_WRITE_FLAGS: &[&str] = &[
    "-d",
    "-D",
    "-m",
    "-M",
    "-c",
    "-C",
    "--delete",
    "--move",
    "--copy",
    "--force",
    "-f",
    "--unset",
    "--unset-all",
    "--add",
    "--replace-all",
    "--edit",
    "-e",
    "--rename-section",
    "--remove-section",
    "add",
    "set-url",
    "remove",
    "rm",
    "rename",
    "prune",
    "push",
    "pop",
    "drop",
    "apply",
    "clear",
    "update",
    "init",
    "deinit",
    "lock",
    "unlock",
    "expire",
    "set-branch",
    "set-head",
    "move",
    "repair",
    "create",
    "save",
];

fn subcommand_allowed(program: &str, args: &[&str]) -> bool {
    let lowered: Vec<String> = args.iter().map(|arg| arg.to_lowercase()).collect();
    let has = |flags: &[&str]| lowered.iter().any(|arg| flags.contains(&arg.as_str()));
    let version_only = |flags: &[&str]| args.len() == 1 && flags.contains(&lowered[0].as_str());
    let sub = lowered.iter().find(|arg| !arg.starts_with('-')).map(String::as_str);
    match program {
        "git" => {
            let Some(sub) = sub else { return false };
            if !GIT_READ_SUBCOMMANDS.contains(&sub) || has(GIT_WRITE_FLAGS) {
                return false;
            }
            let positional: Vec<&str> = lowered.iter().skip_while(|arg| arg.as_str() != sub).skip(1).filter(|arg| !arg.starts_with('-')).map(String::as_str).collect();
            match sub {
                "branch" | "tag" => positional.is_empty() || has(&["-l", "--list", "--contains", "--merged", "--no-merged", "--points-at"]),
                "config" => has(&["--get", "--list", "-l", "--get-all", "--get-regexp"]),
                "stash" => matches!(positional.first(), Some(&"list") | Some(&"show")),
                "submodule" => matches!(positional.first(), Some(&"status") | None),
                "worktree" => matches!(positional.first(), Some(&"list")),
                "remote" => positional.is_empty() || matches!(positional.first(), Some(&"show") | Some(&"get-url")),
                _ => true,
            }
        }
        "mvn" => {
            matches!(sub, Some("dependency:tree" | "dependency:list" | "dependency:analyze" | "help:effective-pom" | "help:describe" | "help:evaluate" | "versions:display-dependency-updates"))
                || version_only(&["-v", "--version"])
        }
        "gradle" | "gradlew" => matches!(sub, Some("dependencies" | "projects" | "tasks" | "properties" | "help")) || version_only(&["-v", "--version"]),
        "cargo" => matches!(sub, Some("metadata" | "tree" | "search")) || version_only(&["-v", "--version"]),
        "npm" | "pnpm" | "yarn" => matches!(sub, Some("ls" | "list" | "view" | "info" | "why" | "outdated" | "root" | "prefix")) || version_only(&["-v", "--version"]),
        "node" | "python" | "python3" | "java" | "javac" | "go" | "dotnet" | "rustc" => version_only(&["-v", "--version", "-version", "version"]),
        _ => true,
    }
}

pub fn check_readonly(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("command is empty".to_string());
    }
    if has_redirect(trimmed) {
        return Err("contains output redirection (> / >> / Out-File); the read-only shell does not allow writing files".to_string());
    }
    if trimmed.contains("$(") && !is_powershell_subexpr_safe(trimmed) {
        return Err("a non-read-only command is nested inside $()".to_string());
    }
    for segment in split_segments(trimmed) {
        let tokens = tokenize(segment);
        if tokens.is_empty() {
            continue;
        }
        let tokens = strip_assignment(&tokens);
        let Some(program) = tokens.first() else {
            continue;
        };
        if tokens.len() == 1 && is_quoted_literal(program) {
            continue;
        }
        let program_name = program_name(program);
        if FORBIDDEN_ANYWHERE.contains(&program_name.as_str()) {
            return Err(format!("`{}` is not a read-only command and cannot be run by the subagent; for modify/install/network operations, tell the main agent in the report", program));
        }
        if !ALLOWED_COMMANDS.contains(&program_name.as_str()) {
            return Err(format!("`{}` is not in the read-only command whitelist (allowed: rg/grep/find/ls/dir/Get-ChildItem/Get-Content/cat/head/tail/wc/tree/git log|show|blame|grep|diff/mvn dependency:tree etc.)", program));
        }
        let args: Vec<&str> = tokens[1..].to_vec();
        if !subcommand_allowed(&program_name, &args) {
            return Err(format!("`{}` only allows read-only subcommands (e.g. git log/show/diff/blame/grep, mvn dependency:tree)", segment.trim()));
        }
        for arg in &args {
            let lowered = program_name_of_arg(arg);
            if FORBIDDEN_ANYWHERE.contains(&lowered.as_str()) {
                return Err(format!("non-read-only command `{}` appears in the arguments", arg));
            }
        }
        if program_name == "sed" && args.iter().any(|arg| arg.starts_with("-i")) {
            return Err("sed -i rewrites files".to_string());
        }
    }
    Ok(())
}

pub fn absolute_paths(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in tokenize(command) {
        let token = token.trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ';' || c == ')' || c == '(');
        let token = token.split_once('=').map(|(_, rhs)| rhs).unwrap_or(token);
        if is_absolute(token) {
            out.push(token.to_string());
        }
    }
    out
}

fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'/' || bytes[2] == b'\\')) || (path.starts_with('/') && path.len() > 1 && !path.starts_with("//"))
}

fn has_redirect(command: &str) -> bool {
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;
    let mut prev: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '>' => {
                    // `2>&1` 这种只是并流, 允许; 其余 > 和 >> 都是写文件
                    let next = chars.peek().copied();
                    let stderr_merge = prev == Some('2') && next == Some('&');
                    if !stderr_merge {
                        return true;
                    }
                }
                _ => {}
            },
        }
        prev = Some(ch);
    }
    false
}

fn is_powershell_subexpr_safe(command: &str) -> bool {
    let mut rest = command;
    while let Some(pos) = rest.find("$(") {
        let inner = &rest[pos + 2..];
        let end = matching_paren(inner).unwrap_or(inner.len());
        let body = inner[..end].trim();
        let is_expression = body.starts_with('$') || body.starts_with('[');
        if is_expression {
            if tokenize(body).iter().any(|token| FORBIDDEN_ANYWHERE.contains(&program_name_of_arg(token).as_str())) {
                return false;
            }
        } else if check_readonly(body).is_err() {
            return false;
        }
        rest = &inner[end..];
    }
    true
}

fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_quoted_literal(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() >= 2 && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"') || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
}

fn split_segments(command: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<char> = None;
    let bytes: Vec<(usize, char)> = command.char_indices().collect();
    let mut index = 0usize;
    while index < bytes.len() {
        let (pos, ch) = bytes[index];
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '|' | ';' | '\n' => {
                    out.push(&command[start..pos]);
                    start = pos + ch.len_utf8();
                    if ch == '|' && index + 1 < bytes.len() && bytes[index + 1].1 == '|' {
                        index += 1;
                        start = bytes[index].0 + 1;
                    }
                }
                '&' if index + 1 < bytes.len() && bytes[index + 1].1 == '&' => {
                    out.push(&command[start..pos]);
                    index += 1;
                    start = bytes[index].0 + 1;
                }
                _ => {}
            },
        }
        index += 1;
    }
    out.push(&command[start..]);
    out.into_iter().filter(|segment| !segment.trim().is_empty()).collect()
}

fn tokenize(segment: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut quote: Option<char> = None;
    for (pos, ch) in segment.char_indices() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
                if start.is_none() {
                    start = Some(pos);
                }
            }
            None if ch.is_whitespace() => {
                if let Some(begin) = start.take() {
                    out.push(&segment[begin..pos]);
                }
            }
            None => {
                if start.is_none() {
                    start = Some(pos);
                }
            }
        }
    }
    if let Some(begin) = start {
        out.push(&segment[begin..]);
    }
    out
}

fn strip_assignment<'a>(tokens: &[&'a str]) -> Vec<&'a str> {
    let mut list: Vec<&str> = tokens.to_vec();
    while let Some(first) = list.first() {
        let first = first.trim_start_matches('(');
        if first.starts_with('$') {
            if list.get(1) == Some(&"=") {
                list.drain(..2);
                continue;
            }
            if first.contains('=') {
                list.remove(0);
                continue;
            }
            // 纯 `$var` 引用当命令是 PowerShell 的调用写法, 不放行
            return Vec::new();
        }
        if !first.starts_with('-') && first.contains('=') && !first.contains('/') && !first.contains('\\') && !first.starts_with('"') {
            list.remove(0);
            continue;
        }
        break;
    }
    list
}

fn program_name(token: &str) -> String {
    let token = token.trim_start_matches(|c| c == '(' || c == '@' || c == '&').trim_matches(|c| c == '"' || c == '\'');
    let token = token.split(')').next().unwrap_or(token);
    let base = token.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(token);
    let base = base.strip_prefix("./").unwrap_or(base);
    base.strip_suffix(".exe").unwrap_or(base).to_lowercase()
}

fn program_name_of_arg(token: &str) -> String {
    program_name(token.trim_matches(|c| c == '"' || c == '\''))
}
