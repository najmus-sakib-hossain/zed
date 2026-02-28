/**
 * Package: pkg-036
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-035';
// import { something } from 'pkg-034';
// import { something } from 'pkg-033';

export function main() {
    console.log('pkg-036 main function');
    return { name: 'pkg-036', version: '1.0.0' };
}

export function helper36() {
    return 36 * 2;
}

export const config = {
    packageNumber: 36,
    totalPackages: 100
};
