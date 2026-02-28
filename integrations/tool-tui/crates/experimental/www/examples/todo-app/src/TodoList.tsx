import { Todo } from './App';
import { TodoItem } from './TodoItem';

interface TodoListProps {
    todos: Todo[];
    onToggle: (id: number) => void;
    onDelete: (id: number) => void;
}

export function TodoList({ todos, onToggle, onDelete }: TodoListProps) {
    if (todos.length === 0) {
        return (
            <div class="empty-state">
                <p>No todos yet. Add one above!</p>
            </div>
        );
    }

    return (
        <ul class="todo-list">
            {todos.map(todo => (
                <TodoItem
                    key={todo.id}
                    todo={todo}
                    onToggle={() => onToggle(todo.id)}
                    onDelete={() => onDelete(todo.id)}
                />
            ))}
        </ul>
    );
}
