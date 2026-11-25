#[path = "../../../../../../termini/ai/src/module/ai/tools/common/edit_replace.rs"]
pub mod edit_replace;

pub mod task {
    pub mod rule {
        #[path = "../../../../../../../../termini/ai/src/module/ai/tools/common/task/rule/scope.rs"]
        pub mod scope;

        #[path = "../../../../../../../../termini/ai/src/module/ai/tools/common/task/rule/read_dedup.rs"]
        pub mod read_dedup;

        #[path = "../../../../../../../../termini/ai/src/module/ai/tools/common/task/rule/search_fanout.rs"]
        pub mod search_fanout;

        #[path = "../../../../../../../../termini/ai/src/module/ai/tools/common/task/rule/search_nomatch.rs"]
        pub mod search_nomatch;

        #[path = "../../../../../../../../termini/ai/src/module/ai/tools/common/task/rule/shell_readonly.rs"]
        pub mod shell_readonly;
    }
}
pub mod web {
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/web/html_text.rs"]
    pub mod html_text;
}
pub mod image {
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/image/transcript.rs"]
    pub mod transcript;
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/image/layout_lines.rs"]
    pub mod layout_lines;
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/image/repeat_guard.rs"]
    pub mod repeat_guard;
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/image/image_prepare.rs"]
    pub mod image_prepare;
}
pub mod document {
    #[path = "../../../../../../../termini/ai/src/module/ai/tools/common/document/external_tool.rs"]
    pub mod external_tool;
}
#[path = "../../../../../../termini/ai/src/module/ai/tools/common/tool_edit_hint.rs"]
pub mod tool_edit_hint;