import { useState } from 'dx';

interface AddTodoProps {
    onAdd: (text: string) => void;
}

export function AddTodo({ onAdd }: AddTodoProps) {
    const [text, setText] = useState('');
    const [error, setError] = useState('');

    const handleSubmit = (e: Event) => {
        e.preventDefault();

        const trimmed = text.trim();
        if (!trimmed) {
            setError('Please enter a todo');
            return;
        }

        if (trimmed.length < 2) {
            setError('Todo must be at least 2 characters');
            return;
        }

        if (trimmed.length > 200) {
            setError('Todo must be less than 200 characters');
            return;
        }

        onAdd(trimmed);
        setText('');
        setError('');
    };

    const handleChange = (e: Event) => {
        const target = e.target as HTMLInputElement;
        setText(target.value);
        if (error) {
            setError('');
        }
    };

    return (
        <form class="add-todo" onSubmit={handleSubmit}>
            <input
                class={`new-todo ${error ? 'has-error' : ''}`}
                type="text"
                placeholder="What needs to be done?"
                value={text}
                onChange={handleChange}
                aria-label="New todo"
                aria-invalid={!!error}
                aria-describedby={error ? 'todo-error' : undefined}
            />
            {error && (
                <span id="todo-error" class="error-message" role="alert">
                    {error}
                </span>
            )}
        </form>
    );
}
