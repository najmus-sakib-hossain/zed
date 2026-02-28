export function About() {
    return (
        <div class="about">
            <h1>About This Blog</h1>

            <section>
                <h2>What is dx-www?</h2>
                <p>
                    dx-www is a high-performance web framework that compiles TSX to
                    optimized binary format. It features automatic runtime selection,
                    tree shaking, and efficient delta patching for updates.
                </p>
            </section>

            <section>
                <h2>Features</h2>
                <ul>
                    <li>TSX/JSX compilation with OXC parser</li>
                    <li>Automatic micro/macro runtime selection</li>
                    <li>Server-side rendering with hydration</li>
                    <li>Sub-20KB WASM client runtime</li>
                    <li>Cross-platform I/O with io_uring, epoll, kqueue, IOCP</li>
                </ul>
            </section>

            <section>
                <h2>Contact</h2>
                <p>
                    Find us on <a href="https://github.com/dx-www/dx-www">GitHub</a> or
                    join our <a href="https://discord.gg/dx-www">Discord community</a>.
                </p>
            </section>
        </div>
    );
}
