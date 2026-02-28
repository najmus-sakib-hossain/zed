import { Todo } from './App';

interface TodoItemProps {
    todo: Todo;
    onToggle: () => void;
    onDelete: () => void;
}

export function TodoItem({ todo, onToggle, onDelete }: TodoItemProps) {
    return (
        <li class={`todo-item ${todo.completed ? 'completed' : ''}`}>
            <div class="view">
                <input
                    class="toggle"
                    type="checkbox"
                    checked={todo.completed}
                    onChange={onToggle}
                    aria-label={`Mark "${todo.text}" as ${todo.completed ? 'incomplete' : 'complete'}`}
                />
                <label onClick={onToggle}>{todo.text}</label>
                <button
                    class="destroy"
                    onClick={onDelete}
                    aria-label={`Delete "${todo.text}"`}
                >
                    ×
                </button>
            </div>
        </li>
    );
}
