/**
 * Dev Iteration Project - Main Entry Point
 * Used for development iteration workflow benchmarks
 */

const express = require('express');
const _ = require('lodash');

const app = express();

app.get('/', (req, res) => {
    res.json({ message: 'Hello World' });
});

app.get('/data', (req, res) => {
    const data = _.range(1, 11).map(n => ({ id: n, value: n * 2 }));
    res.json(data);
});

module.exports = app;
