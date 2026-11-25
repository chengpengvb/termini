连接类型（terminal_kind）：
{{terminal_kind}}

本地终端类型（shell_kind，仅本地终端有值）：
{{shell_kind}}

目标设备（device）：
{{device}}

历史输入（history，按时间理解用户在做什么）：
{{history}}

需要给出的候选条数（suggest_count）：
{{suggest_count}}

终端最近回显（recent_output，含光标所在行全文与光标下方已有行；据此判断刚发生了什么、光标后已有什么、下一步该做什么）：
{{recent_output}}

是否备用屏（alt_screen，true 表示全屏程序；候选按插入文本，不是 shell 命令）：
{{alt_screen}}

当前输入行（current_line，光标之前的原文；候选紧接其末尾拼接，使 current_line + 候选 = 完整结果。已出现过的字符可以再出；不要把末尾已键入且与候选开头重叠的前缀整段再抄一遍）：
{{current_line}}

用户意图补充（instruction，有值时优先满足）：
{{instruction}}

{{web_search_note}}

请先推断用户当前业务意图与任务进度，再给出贴合该意图的候选补全 JSON。
