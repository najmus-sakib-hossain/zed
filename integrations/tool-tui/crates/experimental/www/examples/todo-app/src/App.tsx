import { useState, useEffect } from 'dx';
import { TodoList } from './TodoList';
import { AddTodo } from './AddTodo';
import { Filter } from './Filter';

export interface Todo {
    id: number;
    text: string;
    completed: boolean;
    createdAt: Date;
}

export type FilterType = 'all' | 'active' | 'completed';

const STORAGE_KEY = 'dx-todos';

export default function App() {
    const [todos, setTodos] = useState<Todo[]>([]);
    const [filter, setFilter] = useState<FilterType>('all');

    // Load todos from local storage on mount
    useEffect(() => {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
            try {
                const parsed = JSON.parse(stored);
                setTodos(parsed.map((t: Todo) => ({
                    ...t,
                    createdAt: new Date(t.createdAt)
                })));
            } catch (e) {
                console.error('Failed to load todos:', e);
            }
        }
    }, []);

    // Save todos to local storage on change
    useEffect(() => {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(todos));
    }, [todos]);

    const addTodo = (text: string) => {
        const newTodo: Todo = {
            id: Date.now(),
            text: text.trim(),
            completed: false,
            createdAt: new Date(),
        };
        setTodos([...todos, newTodo]);
    };

    const toggleTodo = (id: number) => {
        setTodos(todos.map(todo =>
            todo.id === id ? { ...todo, completed: !todo.completed } : todo
        ));
    };

    const deleteTodo = (id: number) => {
        setTodos(todos.filter(todo => todo.id !== id));
    };

    const clearCompleted = () => {
        setTodos(todos.filter(todo => !todo.completed));
    };

    const filteredTodos = todos.filter(todo => {
        switch (filter) {
            case 'active':
                return !todo.completed;
            case 'completed':
                return todo.completed;
            default:
                return true;
        }
    });

    const activeCount = todos.filter(t => !t.completed).length;
    const completedCount = todos.filter(t => t.completed).length;

    return (
        <div class="todo-app">
            <header>
                <h1>todos</h1>
            </header>

            <main>
                <AddTodo onAdd={addTodo} />

                <TodoList
                    todos={filteredTodos}
                    onToggle={toggleTodo}
                    onDelete={deleteTodo}
                />

                {todos.length > 0 && (
                    <footer class="todo-footer">
                        <span class="todo-count">
                            {activeCount} {activeCount === 1 ? 'item' : 'items'} left
                        </span>

                        <Filter
                            current={filter}
                            onChange={setFilter}
                        />

                        {completedCount > 0 && (
                            <button
                                class="clear-completed"
                                onClick={clearCompleted}
                            >
                                Clear completed
                            </button>
                        )}
                    </footer>
                )}
            </main>
        </div>
    );
}
