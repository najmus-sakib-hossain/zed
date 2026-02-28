//! TypeScript Compatibility Tests
//!
//! This module tests compatibility with TypeScript, ensuring that
//! the DX package manager can install TypeScript and the runtime
//! can execute the TypeScript compiler.
//!
//! **Validates: Requirements 7.4**

/// TypeScript test scenario
#[derive(Debug, Clone)]
pub struct TypeScriptTestCase {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// TypeScript source code
    pub source: String,
    /// Expected compilation result
    pub expected_result: CompilationResult,
    /// Compiler options (tsconfig.json content)
    pub compiler_options: Option<String>,
}

/// Expected compilation result
#[derive(Debug, Clone)]
pub enum CompilationResult {
    /// Compilation should succeed with expected output
    Success { expected_js: String },
    /// Compilation should fail with expected error
    Error { expected_error: String },
}

/// TypeScript feature category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptFeature {
    /// Basic type annotations
    BasicTypes,
    /// Interfaces
    Interfaces,
    /// Classes
    Classes,
    /// Generics
    Generics,
    /// Enums
    Enums,
    /// Modules
    Modules,
    /// Decorators
    Decorators,
    /// Advanced types (union, intersection, etc.)
    AdvancedTypes,
    /// Type guards
    TypeGuards,
    /// Utility types
    UtilityTypes,
}

/// TypeScript compatibility test suite
pub struct TypeScriptTestSuite {
    test_cases: Vec<TypeScriptTestCase>,
}

impl TypeScriptTestSuite {
    /// Create a new TypeScript test suite
    pub fn new() -> Self {
        let mut suite = Self {
            test_cases: Vec::new(),
        };
        
        suite.add_basic_type_tests();
        suite.add_interface_tests();
        suite.add_class_tests();
        suite.add_generic_tests();
        suite.add_enum_tests();
        suite.add_module_tests();
        suite.add_advanced_type_tests();
        
        suite
    }
    
    fn add_basic_type_tests(&mut self) {
        // Basic type annotations
        self.test_cases.push(TypeScriptTestCase {
            name: "basic_types".to_string(),
            description: "Basic type annotations compile correctly".to_string(),
            source: r#"
const name: string = "John";
const age: number = 30;
const active: boolean = true;
const items: string[] = ["a", "b", "c"];

function greet(name: string): string {
    return `Hello, ${name}!`;
}

console.log(greet(name));
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const name = "John";
const age = 30;
const active = true;
const items = ["a", "b", "c"];
function greet(name) {
    return `Hello, ${name}!`;
}
console.log(greet(name));
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Optional parameters
        self.test_cases.push(TypeScriptTestCase {
            name: "optional_params".to_string(),
            description: "Optional parameters compile correctly".to_string(),
            source: r#"
function greet(name: string, greeting?: string): string {
    return `${greeting || "Hello"}, ${name}!`;
}

console.log(greet("John"));
console.log(greet("Jane", "Hi"));
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function greet(name, greeting) {
    return `${greeting || "Hello"}, ${name}!`;
}
console.log(greet("John"));
console.log(greet("Jane", "Hi"));
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Default parameters
        self.test_cases.push(TypeScriptTestCase {
            name: "default_params".to_string(),
            description: "Default parameters compile correctly".to_string(),
            source: r#"
function greet(name: string, greeting: string = "Hello"): string {
    return `${greeting}, ${name}!`;
}

console.log(greet("John"));
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function greet(name, greeting = "Hello") {
    return `${greeting}, ${name}!`;
}
console.log(greet("John"));
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Type error detection
        self.test_cases.push(TypeScriptTestCase {
            name: "type_error".to_string(),
            description: "Type errors are detected".to_string(),
            source: r#"
const name: string = 123;
"#.to_string(),
            expected_result: CompilationResult::Error {
                expected_error: "Type 'number' is not assignable to type 'string'".to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_interface_tests(&mut self) {
        // Basic interface
        self.test_cases.push(TypeScriptTestCase {
            name: "basic_interface".to_string(),
            description: "Basic interfaces compile correctly".to_string(),
            source: r#"
interface User {
    name: string;
    age: number;
}

function printUser(user: User): void {
    console.log(`${user.name} is ${user.age} years old`);
}

const user: User = { name: "John", age: 30 };
printUser(user);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function printUser(user) {
    console.log(`${user.name} is ${user.age} years old`);
}
const user = { name: "John", age: 30 };
printUser(user);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Optional properties
        self.test_cases.push(TypeScriptTestCase {
            name: "optional_properties".to_string(),
            description: "Optional interface properties compile correctly".to_string(),
            source: r#"
interface Config {
    host: string;
    port?: number;
}

const config: Config = { host: "localhost" };
console.log(config.port || 3000);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const config = { host: "localhost" };
console.log(config.port || 3000);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Readonly properties
        self.test_cases.push(TypeScriptTestCase {
            name: "readonly_properties".to_string(),
            description: "Readonly properties are enforced".to_string(),
            source: r#"
interface Point {
    readonly x: number;
    readonly y: number;
}

const point: Point = { x: 10, y: 20 };
point.x = 5; // Error
"#.to_string(),
            expected_result: CompilationResult::Error {
                expected_error: "Cannot assign to 'x' because it is a read-only property".to_string(),
            },
            compiler_options: None,
        });
        
        // Interface extension
        self.test_cases.push(TypeScriptTestCase {
            name: "interface_extension".to_string(),
            description: "Interface extension compiles correctly".to_string(),
            source: r#"
interface Animal {
    name: string;
}

interface Dog extends Animal {
    breed: string;
}

const dog: Dog = { name: "Rex", breed: "German Shepherd" };
console.log(dog.name, dog.breed);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const dog = { name: "Rex", breed: "German Shepherd" };
console.log(dog.name, dog.breed);
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_class_tests(&mut self) {
        // Basic class
        self.test_cases.push(TypeScriptTestCase {
            name: "basic_class".to_string(),
            description: "Basic classes compile correctly".to_string(),
            source: r#"
class Person {
    name: string;
    age: number;
    
    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }
    
    greet(): string {
        return `Hello, I'm ${this.name}`;
    }
}

const person = new Person("John", 30);
console.log(person.greet());
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
class Person {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }
    greet() {
        return `Hello, I'm ${this.name}`;
    }
}
const person = new Person("John", 30);
console.log(person.greet());
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Access modifiers
        self.test_cases.push(TypeScriptTestCase {
            name: "access_modifiers".to_string(),
            description: "Access modifiers are enforced".to_string(),
            source: r#"
class BankAccount {
    private balance: number;
    
    constructor(initial: number) {
        this.balance = initial;
    }
    
    public getBalance(): number {
        return this.balance;
    }
}

const account = new BankAccount(100);
console.log(account.balance); // Error: private
"#.to_string(),
            expected_result: CompilationResult::Error {
                expected_error: "Property 'balance' is private".to_string(),
            },
            compiler_options: None,
        });
        
        // Class inheritance
        self.test_cases.push(TypeScriptTestCase {
            name: "class_inheritance".to_string(),
            description: "Class inheritance compiles correctly".to_string(),
            source: r#"
class Animal {
    constructor(public name: string) {}
    
    speak(): void {
        console.log(`${this.name} makes a sound`);
    }
}

class Dog extends Animal {
    constructor(name: string, public breed: string) {
        super(name);
    }
    
    speak(): void {
        console.log(`${this.name} barks`);
    }
}

const dog = new Dog("Rex", "German Shepherd");
dog.speak();
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
class Animal {
    constructor(name) {
        this.name = name;
    }
    speak() {
        console.log(`${this.name} makes a sound`);
    }
}
class Dog extends Animal {
    constructor(name, breed) {
        super(name);
        this.breed = breed;
    }
    speak() {
        console.log(`${this.name} barks`);
    }
}
const dog = new Dog("Rex", "German Shepherd");
dog.speak();
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Abstract classes
        self.test_cases.push(TypeScriptTestCase {
            name: "abstract_class".to_string(),
            description: "Abstract classes compile correctly".to_string(),
            source: r#"
abstract class Shape {
    abstract area(): number;
    
    describe(): string {
        return `This shape has area ${this.area()}`;
    }
}

class Circle extends Shape {
    constructor(private radius: number) {
        super();
    }
    
    area(): number {
        return Math.PI * this.radius ** 2;
    }
}

const circle = new Circle(5);
console.log(circle.describe());
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
class Shape {
    describe() {
        return `This shape has area ${this.area()}`;
    }
}
class Circle extends Shape {
    constructor(radius) {
        super();
        this.radius = radius;
    }
    area() {
        return Math.PI * this.radius ** 2;
    }
}
const circle = new Circle(5);
console.log(circle.describe());
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_generic_tests(&mut self) {
        // Generic function
        self.test_cases.push(TypeScriptTestCase {
            name: "generic_function".to_string(),
            description: "Generic functions compile correctly".to_string(),
            source: r#"
function identity<T>(arg: T): T {
    return arg;
}

const num = identity<number>(42);
const str = identity<string>("hello");
console.log(num, str);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function identity(arg) {
    return arg;
}
const num = identity(42);
const str = identity("hello");
console.log(num, str);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Generic class
        self.test_cases.push(TypeScriptTestCase {
            name: "generic_class".to_string(),
            description: "Generic classes compile correctly".to_string(),
            source: r#"
class Container<T> {
    private value: T;
    
    constructor(value: T) {
        this.value = value;
    }
    
    getValue(): T {
        return this.value;
    }
}

const numContainer = new Container<number>(42);
const strContainer = new Container<string>("hello");
console.log(numContainer.getValue(), strContainer.getValue());
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
class Container {
    constructor(value) {
        this.value = value;
    }
    getValue() {
        return this.value;
    }
}
const numContainer = new Container(42);
const strContainer = new Container("hello");
console.log(numContainer.getValue(), strContainer.getValue());
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Generic constraints
        self.test_cases.push(TypeScriptTestCase {
            name: "generic_constraints".to_string(),
            description: "Generic constraints compile correctly".to_string(),
            source: r#"
interface Lengthwise {
    length: number;
}

function logLength<T extends Lengthwise>(arg: T): number {
    return arg.length;
}

console.log(logLength("hello"));
console.log(logLength([1, 2, 3]));
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function logLength(arg) {
    return arg.length;
}
console.log(logLength("hello"));
console.log(logLength([1, 2, 3]));
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_enum_tests(&mut self) {
        // Numeric enum
        self.test_cases.push(TypeScriptTestCase {
            name: "numeric_enum".to_string(),
            description: "Numeric enums compile correctly".to_string(),
            source: r#"
enum Direction {
    Up,
    Down,
    Left,
    Right
}

const dir: Direction = Direction.Up;
console.log(dir);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
var Direction;
(function (Direction) {
    Direction[Direction["Up"] = 0] = "Up";
    Direction[Direction["Down"] = 1] = "Down";
    Direction[Direction["Left"] = 2] = "Left";
    Direction[Direction["Right"] = 3] = "Right";
})(Direction || (Direction = {}));
const dir = Direction.Up;
console.log(dir);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // String enum
        self.test_cases.push(TypeScriptTestCase {
            name: "string_enum".to_string(),
            description: "String enums compile correctly".to_string(),
            source: r#"
enum Color {
    Red = "RED",
    Green = "GREEN",
    Blue = "BLUE"
}

const color: Color = Color.Red;
console.log(color);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
var Color;
(function (Color) {
    Color["Red"] = "RED";
    Color["Green"] = "GREEN";
    Color["Blue"] = "BLUE";
})(Color || (Color = {}));
const color = Color.Red;
console.log(color);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Const enum
        self.test_cases.push(TypeScriptTestCase {
            name: "const_enum".to_string(),
            description: "Const enums are inlined".to_string(),
            source: r#"
const enum Status {
    Active = 1,
    Inactive = 0
}

const status = Status.Active;
console.log(status);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const status = 1 /* Status.Active */;
console.log(status);
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_module_tests(&mut self) {
        // ES module export
        self.test_cases.push(TypeScriptTestCase {
            name: "es_module_export".to_string(),
            description: "ES module exports compile correctly".to_string(),
            source: r#"
export const PI = 3.14159;

export function add(a: number, b: number): number {
    return a + b;
}

export default class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }
}
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
export const PI = 3.14159;
export function add(a, b) {
    return a + b;
}
export default class Calculator {
    add(a, b) {
        return a + b;
    }
}
"#.to_string(),
            },
            compiler_options: Some(r#"{"module": "ESNext"}"#.to_string()),
        });
        
        // Type-only imports
        self.test_cases.push(TypeScriptTestCase {
            name: "type_only_import".to_string(),
            description: "Type-only imports are removed".to_string(),
            source: r#"
import type { User } from './types';

function greet(user: User): string {
    return `Hello, ${user.name}`;
}
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function greet(user) {
    return `Hello, ${user.name}`;
}
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    fn add_advanced_type_tests(&mut self) {
        // Union types
        self.test_cases.push(TypeScriptTestCase {
            name: "union_types".to_string(),
            description: "Union types compile correctly".to_string(),
            source: r#"
type StringOrNumber = string | number;

function format(value: StringOrNumber): string {
    if (typeof value === "string") {
        return value.toUpperCase();
    }
    return value.toFixed(2);
}

console.log(format("hello"));
console.log(format(3.14159));
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
function format(value) {
    if (typeof value === "string") {
        return value.toUpperCase();
    }
    return value.toFixed(2);
}
console.log(format("hello"));
console.log(format(3.14159));
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Intersection types
        self.test_cases.push(TypeScriptTestCase {
            name: "intersection_types".to_string(),
            description: "Intersection types compile correctly".to_string(),
            source: r#"
interface Named {
    name: string;
}

interface Aged {
    age: number;
}

type Person = Named & Aged;

const person: Person = { name: "John", age: 30 };
console.log(person.name, person.age);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const person = { name: "John", age: 30 };
console.log(person.name, person.age);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Mapped types
        self.test_cases.push(TypeScriptTestCase {
            name: "mapped_types".to_string(),
            description: "Mapped types compile correctly".to_string(),
            source: r#"
type Readonly<T> = {
    readonly [P in keyof T]: T[P];
};

interface User {
    name: string;
    age: number;
}

const user: Readonly<User> = { name: "John", age: 30 };
console.log(user.name);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const user = { name: "John", age: 30 };
console.log(user.name);
"#.to_string(),
            },
            compiler_options: None,
        });
        
        // Conditional types
        self.test_cases.push(TypeScriptTestCase {
            name: "conditional_types".to_string(),
            description: "Conditional types compile correctly".to_string(),
            source: r#"
type NonNullable<T> = T extends null | undefined ? never : T;

type A = NonNullable<string | null>;  // string
type B = NonNullable<number | undefined>;  // number

const a: A = "hello";
const b: B = 42;
console.log(a, b);
"#.to_string(),
            expected_result: CompilationResult::Success {
                expected_js: r#"
const a = "hello";
const b = 42;
console.log(a, b);
"#.to_string(),
            },
            compiler_options: None,
        });
    }
    
    /// Get all test cases
    pub fn test_cases(&self) -> &[TypeScriptTestCase] {
        &self.test_cases
    }
    
    /// Get test count
    pub fn test_count(&self) -> usize {
        self.test_cases.len()
    }
}

impl Default for TypeScriptTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_suite_has_tests() {
        let suite = TypeScriptTestSuite::new();
        assert!(!suite.test_cases().is_empty());
    }
    
    #[test]
    fn test_has_basic_type_tests() {
        let suite = TypeScriptTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"basic_types"));
        assert!(names.contains(&"optional_params"));
        assert!(names.contains(&"type_error"));
    }
    
    #[test]
    fn test_has_interface_tests() {
        let suite = TypeScriptTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"basic_interface"));
        assert!(names.contains(&"interface_extension"));
    }
    
    #[test]
    fn test_has_class_tests() {
        let suite = TypeScriptTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"basic_class"));
        assert!(names.contains(&"class_inheritance"));
        assert!(names.contains(&"abstract_class"));
    }
    
    #[test]
    fn test_has_generic_tests() {
        let suite = TypeScriptTestSuite::new();
        let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
        
        assert!(names.contains(&"generic_function"));
        assert!(names.contains(&"generic_class"));
        assert!(names.contains(&"generic_constraints"));
    }
}
