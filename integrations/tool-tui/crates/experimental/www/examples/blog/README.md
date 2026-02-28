
# Blog Example

A blog application demonstrating routing, server-side rendering (SSR), and SEO best practices with dx-www.

## Features

- Multi-page routing with dynamic parameters
- Server-side rendering (SSR) for SEO
- SEO-friendly URLs and meta tags
- Open Graph and Twitter Card support
- Structured data (JSON-LD) for rich snippets
- Semantic HTML with microdata

## Running

```bash
cd examples/blog dx dev ```
Open //localhost:3000 to see the app.


## Building


```bash
dx build ```

## SEO Patterns Demonstrated

### Meta Tags with Head Component

```tsx
import { Head } from 'dx/head';
function Post({ post }) { return ( <> <Head> <title>{post.title} | My Blog</title> <meta name="description" content={post.excerpt} /> <meta property="og:title" content={post.title} /> <link rel="canonical" href={`/post/${post.slug}`} /> </Head> <article>...</article> </> );
}
```

### Structured Data (JSON-LD)

```tsx
const structuredData = { "@context": "https://schema.org", "@type": "BlogPosting", "headline": post.title, "datePublished": post.date, "author": { "@type": "Person", "name": post.author }
};
<Head> <script type="application/ld+json"> {JSON.stringify(structuredData)}
</script> </Head> ```


### Semantic HTML with Microdata


```tsx
<article itemScope itemType="https://schema.org/BlogPosting"> <h1 itemProp="headline">{post.title}</h1> <time itemProp="datePublished">{post.date}</time> <div itemProp="articleBody">{post.content}</div> </article> ```

## SSR Configuration

Enable SSR in `dx.config.json`:
```json
{ "features": { "ssr": true, "hydration": true }
}
```
With SSR enabled: -Search engine crawlers receive fully rendered HTML -Meta tags are present in the initial response -First contentful paint is faster -Client-side hydration enables interactivity

## Project Structure

@tree:blog[]
