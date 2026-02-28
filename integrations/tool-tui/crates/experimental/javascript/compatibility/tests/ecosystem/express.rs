//! Express Compatibility Tests
//!
//! This module tests compatibility with Express.js, the most popular
//! Node.js web framework.
//!
//! **Validates: Requirements 7.3**

use std::collections::HashMap;

/// Express test scenario
#[derive(Debug, Clone)]
pub struct ExpressTestCase {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Server setup code
    pub server_code: String,
    /// HTTP method to test
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// Request body (if any)
    pub body: Option<String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Expected status code
    pub expected_status: u16,
    /// Expected response body (substring match)
    pub expected_body: Option<String>,
    /// Expected headers
    pub expected_headers: HashMap<String, String>,
}

/// HTTP methods for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

/// Express compatibility test suite
pub struct ExpressTestSuite {
    test_cases: Vec<ExpressTestCase>,
}

impl ExpressTestSuite {
    /// Create a new Express test suite
    pub fn new() -> Self {
        let mut suite = Self {
            test_cases: Vec::new(),
        };
        
        suite.add_basic_routing_tests();
        suite.add_middleware_tests();
        suite.add_request_handling_tests();
        suite.add_response_handling_tests();
        suite.add_error_handling_tests();
        
        suite
    }
    
    fn add_basic_routing_tests(&mut self) {
        // Basic GET route
        self.test_cases.push(ExpressTestCase {
            name: "basic_get".to_string(),
            description: "Basic GET route returns correct response".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/', (req, res) => {
                    res.send('Hello World');
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some("Hello World".to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Route with parameters
        self.test_cases.push(ExpressTestCase {
            name: "route_params".to_string(),
            description: "Route parameters are correctly parsed".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/users/:id', (req, res) => {
                    res.json({ id: req.params.id });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/users/123".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"id":"123"}"#.to_string()),
            expected_headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
        });
        
        // Multiple route parameters
        self.test_cases.push(ExpressTestCase {
            name: "multiple_route_params".to_string(),
            description: "Multiple route parameters work correctly".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/users/:userId/posts/:postId', (req, res) => {
                    res.json({
                        userId: req.params.userId,
                        postId: req.params.postId
                    });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/users/42/posts/7".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"userId":"42","postId":"7"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Query parameters
        self.test_cases.push(ExpressTestCase {
            name: "query_params".to_string(),
            description: "Query parameters are correctly parsed".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/search', (req, res) => {
                    res.json({ q: req.query.q, page: req.query.page });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/search?q=test&page=1".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"q":"test","page":"1"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // POST route
        self.test_cases.push(ExpressTestCase {
            name: "basic_post".to_string(),
            description: "POST route handles request body".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.use(express.json());
                
                app.post('/users', (req, res) => {
                    res.status(201).json({ created: true, name: req.body.name });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Post,
            path: "/users".to_string(),
            body: Some(r#"{"name":"John"}"#.to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            expected_status: 201,
            expected_body: Some(r#"{"created":true,"name":"John"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // PUT route
        self.test_cases.push(ExpressTestCase {
            name: "basic_put".to_string(),
            description: "PUT route handles updates".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.use(express.json());
                
                app.put('/users/:id', (req, res) => {
                    res.json({ updated: true, id: req.params.id, name: req.body.name });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Put,
            path: "/users/123".to_string(),
            body: Some(r#"{"name":"Jane"}"#.to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            expected_status: 200,
            expected_body: Some(r#"{"updated":true,"id":"123","name":"Jane"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // DELETE route
        self.test_cases.push(ExpressTestCase {
            name: "basic_delete".to_string(),
            description: "DELETE route handles deletion".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.delete('/users/:id', (req, res) => {
                    res.json({ deleted: true, id: req.params.id });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Delete,
            path: "/users/123".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"deleted":true,"id":"123"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
    }
    
    fn add_middleware_tests(&mut self) {
        // Application-level middleware
        self.test_cases.push(ExpressTestCase {
            name: "app_middleware".to_string(),
            description: "Application-level middleware is executed".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.use((req, res, next) => {
                    req.customHeader = 'middleware-value';
                    next();
                });
                
                app.get('/', (req, res) => {
                    res.json({ header: req.customHeader });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"header":"middleware-value"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Router-level middleware
        self.test_cases.push(ExpressTestCase {
            name: "router_middleware".to_string(),
            description: "Router-level middleware works correctly".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                const router = express.Router();
                
                router.use((req, res, next) => {
                    req.routerMiddleware = true;
                    next();
                });
                
                router.get('/test', (req, res) => {
                    res.json({ routerMiddleware: req.routerMiddleware });
                });
                
                app.use('/api', router);
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/api/test".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some(r#"{"routerMiddleware":true}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Built-in JSON middleware
        self.test_cases.push(ExpressTestCase {
            name: "json_middleware".to_string(),
            description: "express.json() middleware parses JSON body".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.use(express.json());
                
                app.post('/echo', (req, res) => {
                    res.json(req.body);
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Post,
            path: "/echo".to_string(),
            body: Some(r#"{"message":"hello"}"#.to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            expected_status: 200,
            expected_body: Some(r#"{"message":"hello"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // URL-encoded middleware
        self.test_cases.push(ExpressTestCase {
            name: "urlencoded_middleware".to_string(),
            description: "express.urlencoded() middleware parses form data".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.use(express.urlencoded({ extended: true }));
                
                app.post('/form', (req, res) => {
                    res.json(req.body);
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Post,
            path: "/form".to_string(),
            body: Some("name=John&age=30".to_string()),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/x-www-form-urlencoded".to_string());
                h
            },
            expected_status: 200,
            expected_body: Some(r#"{"name":"John","age":"30"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
    }
    
    fn add_request_handling_tests(&mut self) {
        // Request headers
        self.test_cases.push(ExpressTestCase {
            name: "request_headers".to_string(),
            description: "Request headers are accessible".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/headers', (req, res) => {
                    res.json({
                        userAgent: req.get('user-agent'),
                        custom: req.get('x-custom-header')
                    });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/headers".to_string(),
            body: None,
            headers: {
                let mut h = HashMap::new();
                h.insert("x-custom-header".to_string(), "custom-value".to_string());
                h
            },
            expected_status: 200,
            expected_body: Some("custom-value".to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Request IP
        self.test_cases.push(ExpressTestCase {
            name: "request_ip".to_string(),
            description: "Request IP is available".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/ip', (req, res) => {
                    res.json({ ip: req.ip });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/ip".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some("ip".to_string()),
            expected_headers: HashMap::new(),
        });
    }
    
    fn add_response_handling_tests(&mut self) {
        // res.status()
        self.test_cases.push(ExpressTestCase {
            name: "response_status".to_string(),
            description: "res.status() sets correct status code".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/not-found', (req, res) => {
                    res.status(404).json({ error: 'Not found' });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/not-found".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 404,
            expected_body: Some(r#"{"error":"Not found"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // res.set() / res.header()
        self.test_cases.push(ExpressTestCase {
            name: "response_headers".to_string(),
            description: "res.set() sets response headers".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/custom-header', (req, res) => {
                    res.set('X-Custom-Header', 'custom-value');
                    res.send('OK');
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/custom-header".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some("OK".to_string()),
            expected_headers: {
                let mut h = HashMap::new();
                h.insert("x-custom-header".to_string(), "custom-value".to_string());
                h
            },
        });
        
        // res.redirect()
        self.test_cases.push(ExpressTestCase {
            name: "response_redirect".to_string(),
            description: "res.redirect() sends redirect response".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/old-path', (req, res) => {
                    res.redirect('/new-path');
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/old-path".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 302,
            expected_body: None,
            expected_headers: {
                let mut h = HashMap::new();
                h.insert("location".to_string(), "/new-path".to_string());
                h
            },
        });
        
        // res.type()
        self.test_cases.push(ExpressTestCase {
            name: "response_type".to_string(),
            description: "res.type() sets content-type".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/html', (req, res) => {
                    res.type('html').send('<h1>Hello</h1>');
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/html".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 200,
            expected_body: Some("<h1>Hello</h1>".to_string()),
            expected_headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "text/html".to_string());
                h
            },
        });
    }
    
    fn add_error_handling_tests(&mut self) {
        // 404 handler
        self.test_cases.push(ExpressTestCase {
            name: "404_handler".to_string(),
            description: "404 handler catches unmatched routes".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/', (req, res) => {
                    res.send('Home');
                });
                
                app.use((req, res) => {
                    res.status(404).json({ error: 'Route not found' });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/nonexistent".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 404,
            expected_body: Some(r#"{"error":"Route not found"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
        
        // Error middleware
        self.test_cases.push(ExpressTestCase {
            name: "error_middleware".to_string(),
            description: "Error middleware catches thrown errors".to_string(),
            server_code: r#"
                const express = require('express');
                const app = express();
                
                app.get('/error', (req, res, next) => {
                    next(new Error('Something went wrong'));
                });
                
                app.use((err, req, res, next) => {
                    res.status(500).json({ error: err.message });
                });
                
                app.listen(3000);
            "#.to_string(),
            method: HttpMethod::Get,
            path: "/error".to_string(),
            body: None,
            headers: HashMap::new(),
            expected_status: 500,
            expected_body: Some(r#"{"error":"Something went wrong"}"#.to_string()),
            expected_headers: HashMap::new(),
        });
    }
    
    /// Get all test cases
    pub fn test_cases(&self) -> &[ExpressTestCase] {
        &self.test_cases
    }
    
    /// Get test count
    pub fn test_count(&self) -> usize {
        self.test_cases.len()
    }
}

impl Default for ExpressTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_suite_has_tests() {
        let suite = ExpressTestSuite::new();
        assert!(!suite.test_cases().is_empty());
    }
    
    #[test]
    fn test_has_basic_routing_tests() {
        let suite = ExpressTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"basic_get"));
        assert!(names.contains(&"route_params"));
        assert!(names.contains(&"basic_post"));
    }
    
    #[test]
    fn test_has_middleware_tests() {
        let suite = ExpressTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"app_middleware"));
        assert!(names.contains(&"json_middleware"));
    }
    
    #[test]
    fn test_has_error_handling_tests() {
        let suite = ExpressTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"404_handler"));
        assert!(names.contains(&"error_middleware"));
    }
}
