import { Link } from 'dx/router';
import { Post } from '../data/posts';

interface PostCardProps {
    post: Post;
}

export function PostCard({ post }: PostCardProps) {
    return (
        <article class="post-card">
            <h3>
                <Link to={`/post/${post.slug}`}>{post.title}</Link>
            </h3>
            <p class="excerpt">{post.excerpt}</p>
            <div class="meta">
                <time dateTime={post.date}>{formatDate(post.date)}</time>
                <span class="author">{post.author}</span>
            </div>
            {post.tags && (
                <div class="tags">
                    {post.tags.slice(0, 3).map(tag => (
                        <span key={tag} class="tag">{tag}</span>
                    ))}
                </div>
            )}
        </article>
    );
}

function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
    });
}
