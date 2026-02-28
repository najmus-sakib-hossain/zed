use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize)]
struct FunctionCallRequest {
    contents: Vec<Content>,
    tools: Vec<Tool>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize, Clone)]
struct Tool {
    function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize, Clone)]
struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct FunctionCallResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ResponsePart {
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    Text {
        text: String,
    },
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    args: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 Gemma 3 27B Function Calling Demo\n");

    // Get API key from environment
    let api_key = std::env::var("GOOGLE_AI_STUDIO_KEY")
        .expect("Please set GOOGLE_AI_STUDIO_KEY environment variable");

    // Define functions
    let tools = vec![Tool {
        function_declarations: vec![
            FunctionDeclaration {
                name: "get_weather".to_string(),
                description: "Get the current weather for a location".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state, e.g. San Francisco, CA"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "description": "The temperature unit"
                        }
                    },
                    "required": ["location"]
                }),
            },
            FunctionDeclaration {
                name: "calculate".to_string(),
                description: "Perform a mathematical calculation".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate"
                        }
                    },
                    "required": ["expression"]
                }),
            },
        ],
    }];

    // Test 1: Weather function call
    println!("📍 Test 1: Weather Query");
    println!("User: What's the weather in Tokyo?");

    let request = FunctionCallRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: "What's the weather in Tokyo?".to_string(),
            }],
        }],
        tools: tools.clone(),
    };

    call_api(&api_key, "gemini-2.5-flash", &request).await?;

    println!("\n---\n");

    // Test 2: Math calculation
    println!("🔢 Test 2: Math Calculation");
    println!("User: Calculate 25 * 48 + 137");

    let request = FunctionCallRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: "Calculate 25 * 48 + 137".to_string(),
            }],
        }],
        tools: tools.clone(),
    };

    call_api(&api_key, "gemini-2.5-flash", &request).await?;

    println!("\n---\n");

    // Test 3: Multiple function calls
    println!("🌍 Test 3: Multiple Queries");
    println!("User: What's the weather in London and Paris? Also calculate 100 / 5");

    let request = FunctionCallRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: "What's the weather in London and Paris? Also calculate 100 / 5".to_string(),
            }],
        }],
        tools,
    };

    call_api(&api_key, "gemini-2.5-flash", &request).await?;

    Ok(())
}

async fn call_api(api_key: &str, model: &str, request: &FunctionCallRequest) -> Result<()> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::new();
    let response = client.post(&url).json(request).send().await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        eprintln!("❌ Error: {}", error);
        return Ok(());
    }

    let response_text = response.text().await?;
    let result: FunctionCallResponse = serde_json::from_str(&response_text)?;

    if let Some(candidate) = result.candidates.first() {
        for part in &candidate.content.parts {
            match part {
                ResponsePart::Text { text } => {
                    println!("💬 AI Response: {}", text);
                }
                ResponsePart::FunctionCall { function_call } => {
                    println!("🔧 Function Call Detected:");
                    println!("   Function: {}", function_call.name);
                    println!(
                        "   Arguments: {}",
                        serde_json::to_string_pretty(&function_call.args)?
                    );

                    // Simulate function execution
                    match function_call.name.as_str() {
                        "get_weather" => {
                            let location =
                                function_call.args["location"].as_str().unwrap_or("Unknown");
                            println!("   ✅ Executing: Getting weather for {}", location);
                            println!("   📊 Result: Sunny, 22°C");
                        }
                        "calculate" => {
                            let expr = function_call.args["expression"].as_str().unwrap_or("0");
                            println!("   ✅ Executing: Calculating {}", expr);
                            println!("   📊 Result: 1337");
                        }
                        _ => {
                            println!("   ⚠️  Unknown function");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
