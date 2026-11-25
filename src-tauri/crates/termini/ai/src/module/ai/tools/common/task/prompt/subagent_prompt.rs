use crate::module::ai::tools::common::task::rule::scope;

pub struct Environment {
    pub os: String,
    pub terminal: String,
}

pub fn build(scope: &[String], requested: &[String], environment: Option<&Environment>, background: &str, prompt: &str) -> String {
    let mut head = String::new();
    head.push_str("[子任务]\n你是主助手派出的子任务代理, 与主助手同一套工具和审批档位: 需要审批的操作照常发起, 会弹给用户决定, 被跳过就换路或写进报告。看不见父会话历史, 任务以下面的 prompt 为准, 不要自己改题; 不能再派 task。做完交一份报告: 做了什么、依据(路径:行号)、没做成的和需要主助手接手的。报告和过程中所有给用户看的文字遵循 [输出语言]: [背景参考] 里「用户的原始问题」就是用户原话, 以它的语言为准, 不要跟着任务 prompt 的语言走。\n\n");
    if !scope.is_empty() {
        head.push_str("[任务范围]\n只允许访问以下目录, 工具的 root / cwd / files.root 直接填这些绝对路径, 不要用相对路径, 也不要去范围外的目录:\n");
        for dir in scope {
            head.push_str("- ");
            head.push_str(dir);
            head.push('\n');
        }
        if let Some(common) = scope::common_ancestor(scope) {
            head.push_str(&format!("这些目录的公共上级是 {}(仅供拼路径参考, 不是允许范围)。\n", common));
        }
        let files: Vec<&String> = requested.iter().filter(|item| !scope.iter().any(|dir| scope::normalize(dir) == scope::normalize(item))).collect();
        if !files.is_empty() {
            head.push_str("调用方点名的文件(先直接批量读完它们, 再决定还要不要搜):\n");
            for file in files {
                head.push_str(&format!("- {}\n", file));
            }
        }
        head.push('\n');
    }
    if let Some(environment) = environment {
        head.push_str(&format!("[运行环境]\n操作系统 {}, 本机终端是 {}, 命令语法必须按它写。\n\n", environment.os, environment.terminal));
    }
    if !background.trim().is_empty() {
        head.push_str(&format!("[背景参考]\n以下是主代理已经掌握的情况, 只用来避免重复劳动和理解问题来源, 不是你的结论, 以你自己读到的代码为准:\n{}\n", background));
    }
    format!("{}[任务]\n{}", head, prompt)
}
