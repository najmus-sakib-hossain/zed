import { Head } from 'dx/head';
import { posts } from '../data/posts';
import { PostCard } from '../components/PostCard';

export function Home() {
    return (
        <>
            {/* SEO Meta Tags for Home Page */}
            <Head>
                <title>dx-www Blog | Web Development with Rust</title>
                <meta name="description" content="Thoughts on web development, Rust, and dx-www - the high-performance web framework." />
                <meta name="keywords" content="dx-www, rust, web development, tsx, jsx, ssr" />

                {/* Open Graph */}
                <meta property="og:title" content="dx-www Blog" />
                <meta property="og:description" content="Thoughts on web development, Rust, and dx-www." />
                <meta property="og:type" content="website" />
                <meta property="og:url" content="https://example.com/" />

                {/* Twitter Card */}
                <meta name="twitter:card" content="summary" />
                <meta name="twitter:title" content="dx-www Blog" />
                <meta name="twitter:description" content="Thoughts on web development, Rust, and dx-www." />

                <link rel="canonical" href="https://example.com/" />
            </Head>

            <div class="home">
                <section class="hero">
                    <h1>Welcome to the Blog</h1>
                    <p>Thoughts on web development, Rust, and dx-www.</p>
                </section>

                <section class="posts">
                    <h2>Recent Posts</h2>
                    <div class="post-grid">
                        {posts.map(post => (
                            <PostCard key={post.slug} post={post} />
                        ))}
                    </div>
                </section>
            </div>
        </>
    );
}
