import { Router, Route } from 'dx/router';
import { Layout } from './components/Layout';
import { Home } from './pages/Home';
import { Post } from './pages/Post';
import { About } from './pages/About';
import { NotFound } from './pages/NotFound';

export default function App() {
    return (
        <Router>
            <Layout>
                <Route path="/" component={Home} />
                <Route path="/post/:slug" component={Post} />
                <Route path="/about" component={About} />
                <Route path="*" component={NotFound} />
            </Layout>
        </Router>
    );
}
