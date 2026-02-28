import { useParams, Link } from 'dx/router';
import { Head } from 'dx/head';
import { posts } from '../data/posts';

export function Post() {
    const { slug } = useParams<{ slug: string }>();
    const post = posts.find(p => p.slug === slug);

    if (!post) {
        return (
            <>
                <Head>
                    <title>Post Not Found | dx-www Blog</title>
                    <meta name="robots" content="noindex" />
                </Head>
                <div class="not-found">
                    <h1>Post Not Found</h1>
                    <p>The post you're looking for doesn't exist.</p>
                    <Link to="/">Back to Home</Link>
                </div>
            </>
        );
    }

    // SEO: Generate structured data for search engines
    const structuredData = {
        "@context": "https://schema.org",
        "@type": "BlogPosting",
        "headline": post.title,
        "description": post.excerpt,
        "datePublished": post.date,
        "author": {
            "@type": "Person",
            "name": post.author
        },
        "keywords": post.tags?.join(", ")
    };

    return (
        <>
            {/* SEO Meta Tags - rendered server-side for crawlers */}
            <Head>
                <title>{post.title} | dx-www Blog</title>
                <meta name="description" content={post.excerpt} />
                <meta name="author" content={post.author} />
                {post.tags && <meta name="keywords" content={post.tags.join(", ")} />}

                {/* Open Graph tags for social sharing */}
                <meta property="og:title" content={post.title} />
                <meta property="og:description" content={post.excerpt} />
                <meta property="og:type" content="article" />
                <meta property="og:url" content={`https://example.com/post/${post.slug}`} />
                <meta property="article:published_time" content={post.date} />
                <meta property="article:author" content={post.author} />

                {/* Twitter Card tags */}
                <meta name="twitter:card" content="summary" />
                <meta name="twitter:title" content={post.title} />
                <meta name="twitter:description" content={post.excerpt} />

                {/* Canonical URL for SEO */}
                <link rel="canonical" href={`https://example.com/post/${post.slug}`} />

                {/* Structured data for rich snippets */}
                <script type="application/ld+json">
                    {JSON.stringify(structuredData)}
                </script>
            </Head>

            <article class="post" itemScope itemType="https://schema.org/BlogPosting">
                <header>
                    <h1 itemProp="headline">{post.title}</h1>
                    <div class="meta">
                        <time dateTime={post.date} itemProp="datePublished">
                            {formatDate(post.date)}
                        </time>
                        <span class="author" itemProp="author">{post.author}</span>
                    </div>
                    {post.tags && (
                        <div class="tags">
                            {post.tags.map(tag => (
                                <span key={tag} class="tag" itemProp="keywords">{tag}</span>
                            ))}
                        </div>
                    )}
                </header>

                <div class="content" itemProp="articleBody">
                    {post.content.split('\n\n').map((paragraph, i) => (
                        <p key={i}>{paragraph}</p>
                    ))}
                </div>

                <footer>
                    <Link to="/" class="back-link">← Back to all posts</Link>
                </footer>
            </article>
        </>
    );
}

function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
    });
}
