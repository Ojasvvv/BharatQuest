/**
 * Apatheia Real Self-Healing Demo
 * 
 * This script runs a genuine self-healing loop by prompting Groq's Llama 3 
 * to write a JavaScript function, sending the raw code to the live Apatheia 
 * sandbox, and feeding back any actual runtime errors to the LLM for a retry.
 */

const GROQ_API_KEY = process.env.GROQ_API_KEY;
const APATHEIA_API_KEY = process.env.APATHEIA_API_KEY;
const APATHEIA_URL = process.env.APATHEIA_URL || "https://bharatquest.onrender.com/v1/execute";
const MAX_ITERATIONS = 3;

if (!GROQ_API_KEY || !APATHEIA_API_KEY) {
    console.error("\n❌ ERROR: Missing required environment variables.");
    console.error("Please ensure GROQ_API_KEY and APATHEIA_API_KEY are set.\n");
    process.exit(1);
}

// Colors for terminal output
const colors = {
    reset: "\x1b[0m",
    bright: "\x1b[1m",
    green: "\x1b[32m",
    yellow: "\x1b[33m",
    red: "\x1b[31m",
    cyan: "\x1b[36m",
    magenta: "\x1b[35m"
};

const SYSTEM_PROMPT = `You are an AI coding agent. You must write a JavaScript function.
Write the FASTEST, FIRST INSTINCT implementation. Do not be overly defensive.
Do not add try/catch blocks unless explicitly asked.
Make assumptions if needed.
Return ONLY valid JavaScript code. NO markdown formatting, NO \`\`\`javascript wrapping, NO explanations.`;

const USER_PROMPT = `Write a JS function called 'processData' that takes an array of strings, 
parses them as JSON, and returns the sum of the 'value' fields. 
Then, call processData(['{"value": 10}', '{"value": 20}', 'invalid-json']) and return the result.`;

async function callGroq(messages) {
    const response = await fetch("https://api.groq.com/openai/v1/chat/completions", {
        method: "POST",
        headers: {
            "Authorization": `Bearer ${GROQ_API_KEY}`,
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            model: "llama-3.3-70b-versatile",
            messages: messages,
            temperature: 0.7
        })
    });

    if (!response.ok) {
        throw new Error(`Groq API Error: ${response.status} ${await response.text()}`);
    }

    const data = await response.json();
    let code = data.choices[0].message.content.trim();
    
    // Strip markdown fencing if the LLM ignored instructions
    if (code.startsWith("```")) {
        const lines = code.split("\n");
        if (lines[0].startsWith("```")) lines.shift();
        if (lines[lines.length - 1].startsWith("```")) lines.pop();
        code = lines.join("\n");
    }
    
    return code;
}

async function runApatheia(code, parentRequestId = null) {
    const payload = {
        request_id: `demo-${Date.now()}`,
        language: "javascript",
        code: code,
        timeout_ms: 5000,
        memory_limit_mb: 64
    };

    if (parentRequestId) {
        payload.parent_request_id = parentRequestId;
    }

    const response = await fetch(APATHEIA_URL, {
        method: "POST",
        headers: {
            "X-API-Key": APATHEIA_API_KEY,
            "Content-Type": "application/json"
        },
        body: JSON.stringify(payload)
    });

    if (!response.ok) {
        if (response.status === 429) {
            console.log(`${colors.yellow}⚠️ Rate limited. Waiting 2 seconds...${colors.reset}`);
            await new Promise(r => setTimeout(r, 2000));
            return runApatheia(code, parentRequestId); // retry
        }
        throw new Error(`Apatheia API Error: ${response.status} ${await response.text()}`);
    }

    return response.json();
}

function printMetrics(metrics) {
    console.log(`\n${colors.cyan}[METRICS]${colors.reset}`);
    console.log(`  ├─ Clone Time:    ${metrics.instance_clone_time_us} μs`);
    console.log(`  ├─ Exec Time:     ${metrics.execution_time_us} μs`);
    console.log(`  ├─ Total Time:    ${metrics.total_time_us} μs`);
    console.log(`  └─ Fuel Consumed: ${metrics.fuel_consumed}`);
}

async function main() {
    console.log(`\n${colors.bright}${colors.magenta}=== APATHEIA SELF-HEALING DEMO ===${colors.reset}\n`);

    const messages = [
        { role: "system", content: SYSTEM_PROMPT },
        { role: "user", content: USER_PROMPT }
    ];

    let parentRequestId = `demo-parent-${Date.now()}`;
    let iteration = 1;

    while (iteration <= MAX_ITERATIONS) {
        console.log(`${colors.bright}--- ITERATION ${iteration} ---${colors.reset}`);
        console.log(`${colors.yellow}🧠 Asking Groq (Llama 3) to write code...${colors.reset}`);
        
        let code;
        try {
            code = await callGroq(messages);
        } catch (e) {
            console.error(`${colors.red}Failed to call Groq: ${e.message}${colors.reset}`);
            break;
        }

        console.log(`\n${colors.bright}📝 Generated Code:${colors.reset}\n${code}\n`);
        console.log(`${colors.yellow}⚡ Sending to Apatheia Execution Engine...${colors.reset}`);

        let result;
        try {
            result = await runApatheia(code, parentRequestId);
        } catch (e) {
            console.error(`${colors.red}Failed to call Apatheia: ${e.message}${colors.reset}`);
            break;
        }

        if (result.status === "success") {
            console.log(`\n${colors.green}✅ SUCCESS${colors.reset}`);
            console.log(`${colors.bright}Stdout:${colors.reset}\n${result.stdout}`);
            printMetrics(result.metrics);
            break;
        } else if (result.status === "runtime_error") {
            console.log(`\n${colors.red}❌ RUNTIME ERROR${colors.reset}`);
            console.log(`${colors.bright}Error Message:${colors.reset} ${result.error_telemetry.message}`);
            printMetrics(result.metrics);
            
            console.log(`\n${colors.yellow}🔄 Feeding error back to LLM...${colors.reset}\n`);
            
            // Add Assistant's code to conversation
            messages.push({ role: "assistant", content: code });
            // Add Apatheia's generated feedback prompt to conversation
            messages.push({ role: "system", content: result.llm_feedback_prompt.content });
            
            iteration++;
        } else if (result.status === "rejected") {
            console.log(`\n${colors.red}🛑 REJECTED (Hard Stop)${colors.reset}`);
            console.log(`${colors.bright}Reason:${colors.reset} ${result.reason}`);
            printMetrics(result.metrics);
            console.log(`\n${colors.red}Fatal rejection. Halting self-healing loop.${colors.reset}\n`);
            break;
        }
    }

    if (iteration > MAX_ITERATIONS) {
        console.log(`\n${colors.red}💀 GAVE UP: Exhausted maximum iterations (${MAX_ITERATIONS}).${colors.reset}\n`);
    }
}

main().catch(console.error);
