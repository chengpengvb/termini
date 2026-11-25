import {Box} from '@mui/material';
import {Ref, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {Cancel, CheckCircleOutline, RadioButtonChecked, RadioButtonUnchecked} from '@mui/icons-material';

interface TodoItemView {
    content: string;
    status: string;
}

interface TodoArgsRaw {
    todos?: TodoItemView[];
}

interface ItemTodoWriteProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

const TODO_STATUS = {
    PENDING: 'pending',
    IN_PROGRESS: 'in_progress',
    COMPLETED: 'completed',
    CANCELLED: 'cancelled',
} as const;

export function ItemTodoWrite(props: ItemTodoWriteProps) {
    const {ref} = props;
    const {t} = useI18n();
    const [todos, setTodos] = useState<TodoItemView[]>([]);

    function resolveItemClass(status: string): string {
        if (status === TODO_STATUS.COMPLETED) {
            return 'tm-agent-todo-item is-completed';
        }
        if (status === TODO_STATUS.IN_PROGRESS) {
            return 'tm-agent-todo-item is-in-progress';
        }
        if (status === TODO_STATUS.CANCELLED) {
            return 'tm-agent-todo-item is-cancelled';
        }
        return 'tm-agent-todo-item';
    }

    function renderIcon(status: string) {
        if (status === TODO_STATUS.COMPLETED) {
            return <CheckCircleOutline className="tm-agent-todo-icon is-completed" sx={{fontSize: 14}}/>;
        }
        if (status === TODO_STATUS.IN_PROGRESS) {
            return <RadioButtonChecked className="tm-agent-todo-icon is-in-progress" sx={{fontSize: 14}}/>;
        }
        if (status === TODO_STATUS.CANCELLED) {
            return <Cancel className="tm-agent-todo-icon is-cancelled" sx={{fontSize: 14}}/>;
        }
        return <RadioButtonUnchecked className="tm-agent-todo-icon" sx={{fontSize: 14}}/>;
    }

    function onEvent(eventItem: AiChatItem): void {
        if (eventItem.type !== RESP_EVENT_AGENT.ON_TOOL_CALL) {
            return;
        }
        const args = eventItem.context.args as TodoArgsRaw | null;
        if (isNotNil(args) && Array.isArray(args.todos)) {
            setTodos(args.todos);
        }
    }

    useItemHandle(ref, onEvent);

    if (todos.length === 0) {
        return null;
    }

    return (
        <Box className="tm-agent-tool-card">
            <Box className="tm-agent-tool-head">
                <Box component="span" className="tm-agent-tool-title">{t('agent.tool.todoTitle')}</Box>
            </Box>
            <Box className="tm-agent-todo-list" component="ul">
                {todos.map((todo, index) => (
                    <Box key={`${index}-${todo.content}`} className={resolveItemClass(todo.status)} component="li">
                        {renderIcon(todo.status)}
                        <Box component="span" className="tm-agent-todo-content">{todo.content}</Box>
                    </Box>
                ))}
            </Box>
        </Box>
    );
}
