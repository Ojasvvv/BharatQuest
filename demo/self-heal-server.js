const http = require('http');
const fs = require('fs');
const path = require('path');

const GROQ_API_KEY = process.env.GROQ_API_KEY;
const APATHEIA_API_KEY = process.env.APATHEIA_API_KEY;
const APATHEIA_URL = process.env.APATHEIA_URL || "https://bharatquest.onrender.com/v1/execute";
const MAX_ITERATIONS = 3;

if (!GROQ_API_KEY || !APATHEIA_API_KEY) {
    console.error("\n❌ ERROR: Missing required environment variables.");
    console.error("Please ensure GROQ_API_KEY and APATHEIA_API_KEY are set.\n");
    process.exit(1);
}

const SYSTEM_PROMPT = `You are an AI coding agent. You must write a JavaScript function.
Write the FASTEST, FIRST INSTINCT implementation. Do not be overly defensive.
Do not add try/catch blocks unless explicitly asked.
Make assumptions if needed.
Return ONLY valid JavaScript code. NO markdown formatting, NO \`\`\`javascript wrapping, NO explanations.`;

const PROMPTS = {
    realistic_bug: {
        taskText: `Write a JS function called 'sumData' that takes an array of values and returns their sum.\nThen, call sumData([10, 20, 30]) and return the result.`,
        llmText: `Write a JS function called 'sumData' that takes an array of values and returns their sum. 
For the sake of this test, INTENTIONALLY write a version that throws a ReferenceError ON YOUR FIRST ATTEMPT ONLY by attempting to use an undeclared variable inside the loop. If you receive error feedback in a subsequent message, you MUST fix the error and write correct code.
Then, call sumData([10, 20, 30]) and return the result.`
    },
    dangerous: {
        taskText: `Write the simplest possible JavaScript loop to find all prime numbers up to 50000000, print the count, and return the count.`,
        llmText: `Write the simplest possible JavaScript loop to find all prime numbers up to 50000000, print the count, and return the count.`
    }
};

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
            await new Promise(r => setTimeout(r, 2000));
            return runApatheia(code, parentRequestId);
        }
        throw new Error(`Apatheia API Error: ${response.status} ${await response.text()}`);
    }

    return response.json();
}

const server = http.createServer((req, res) => {
    res.setHeader('Access-Control-Allow-Origin', '*');

    if (req.method === 'OPTIONS') {
        res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
        res.writeHead(204);
        res.end();
        return;
    }

    if (req.url.startsWith('/start')) {
        const url = new URL(req.url, `http://${req.headers.host}`);
        const mode = url.searchParams.get('mode') || 'realistic_bug';

        if (!PROMPTS[mode]) {
            res.writeHead(400);
            res.end("Invalid mode");
            return;
        }

        res.writeHead(200, {
            'Content-Type': 'text/event-stream',
            'Cache-Control': 'no-cache',
            'Connection': 'keep-alive'
        });

        const sendEvent = (type, data) => {
            res.write(`event: ${type}\n`);
            res.write(`data: ${JSON.stringify(data)}\n\n`);
        };

        const runLoop = async () => {
            const messages = [
                { role: "system", content: SYSTEM_PROMPT },
                { role: "user", content: PROMPTS[mode].llmText }
            ];

            let parentRequestId = `demo-parent-${Date.now()}`;
            let iteration = 1;
            let maxIters = mode === 'dangerous' ? 1 : MAX_ITERATIONS;

            sendEvent('start', { mode, task: PROMPTS[mode].taskText });

            while (iteration <= maxIters) {
                sendEvent('llm_generating', { iteration });
                
                let code;
                try {
                    code = await callGroq(messages);
                } catch (e) {
                    sendEvent('error', { error: `Groq error: ${e.message}` });
                    break;
                }

                sendEvent('code_generated', { iteration, code });
                sendEvent('apatheia_evaluating', { iteration });

                let result;
                try {
                    result = await runApatheia(code, parentRequestId);
                } catch (e) {
                    sendEvent('error', { error: `Apatheia error: ${e.message}` });
                    break;
                }

                sendEvent('apatheia_result', { iteration, result });

                if (result.status === "success") {
                    sendEvent('completed', { reason: 'success' });
                    break;
                } else if (result.status === "runtime_error") {
                    if (mode === 'dangerous') {
                        // Shouldn't happen ideally if it times out, but just in case
                        sendEvent('completed', { reason: 'runtime_error_aborted' });
                        break;
                    }
                    
                    messages.push({ role: "assistant", content: code });
                    messages.push({ role: "system", content: result.llm_feedback_prompt.content });
                    
                    sendEvent('retry_feedback', { iteration, feedback: result.llm_feedback_prompt.content });
                    iteration++;
                } else if (result.status === "rejected") {
                    sendEvent('completed', { reason: 'rejected' });
                    break; // Hard stop
                }
            }

            if (iteration > maxIters && mode !== 'dangerous') {
                sendEvent('completed', { reason: 'max_iterations_exhausted' });
            }
            res.end();
        };

        runLoop().catch(e => {
            sendEvent('error', { error: e.message });
            res.end();
        });
        
        req.on('close', () => {
            // Client disconnected early
        });
        return;
    }

    if (req.method === 'GET') {
        if (req.url === '/health') {
            res.writeHead(200, { 'Content-Type': 'text/plain' });
            res.end('ok');
            return;
        }

        let filePath = req.url === '/' ? '/index.html' : req.url;
        // strip query params for file lookup
        filePath = filePath.split('?')[0];
        
        const extname = String(path.extname(filePath)).toLowerCase();
        const mimeTypes = {
            '.html': 'text/html',
            '.js': 'text/javascript',
            '.css': 'text/css',
            '.json': 'application/json',
            '.png': 'image/png',
            '.jpg': 'image/jpg',
            '.gif': 'image/gif',
            '.svg': 'image/svg+xml',
            '.wav': 'audio/wav',
            '.mp4': 'video/mp4',
            '.woff': 'application/font-woff',
            '.ttf': 'application/font-ttf',
            '.eot': 'application/vnd.ms-fontobject',
            '.otf': 'application/font-otf',
            '.wasm': 'application/wasm'
        };

        const contentType = mimeTypes[extname] || 'application/octet-stream';
        const absolutePath = path.join(__dirname, '../dashboard/dist', filePath);

        fs.readFile(absolutePath, (error, content) => {
            if (error) {
                if (error.code === 'ENOENT') {
                    fs.readFile(path.join(__dirname, '../dashboard/dist/index.html'), (err, content) => {
                        if (err) {
                            res.writeHead(500);
                            res.end('Error loading index.html');
                        } else {
                            res.writeHead(200, { 'Content-Type': 'text/html' });
                            res.end(content, 'utf-8');
                        }
                    });
                } else {
                    res.writeHead(500);
                    res.end(`Server Error: ${error.code}`);
                }
            } else {
                res.writeHead(200, { 'Content-Type': contentType });
                res.end(content, 'utf-8');
            }
        });
        return;
    }

    res.writeHead(404);
    res.end();
});

const PORT = process.env.PORT || 3001;
server.listen(PORT, () => {
    console.log(`Live Agent Demo SSE server listening on port ${PORT}`);
});
