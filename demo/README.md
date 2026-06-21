# Apatheia Real Self-Healing Demo

This directory contains a genuine, end-to-end self-healing agent demonstration. It connects to the real Groq LLM API to write code and sends it to the real live Apatheia server for execution. This is not simulated; any errors and retries happen precisely as they would in production.

## Requirements
*   Node.js 18+ (for built-in `fetch`)

## Environment Variables
You must set the following environment variables before running the script:
*   `GROQ_API_KEY`: A valid API key for the Groq completions API.
*   `APATHEIA_API_KEY`: The live API key for the Apatheia deployment.
*   `APATHEIA_URL`: (Optional) Defaults to `https://bharatquest.onrender.com/v1/execute`. Change this if testing against `http://127.0.0.1:8080/v1/execute`.

## How to Run

To run the live agent demo with the React dashboard:

1. **Set Environment Variables**:
   ```bash
   export GROQ_API_KEY="your-groq-key"
   export APATHEIA_API_KEY="your-apatheia-key"
   # Optional: export APATHEIA_URL="http://127.0.0.1:8080/v1/execute"
   ```

2. **Start the Demo SSE Server**:
   ```bash
   node demo/self-heal-server.js
   ```
   *This starts a local zero-dependency Node server on port 3001 that streams the agent loop.*

3. **Start the Dashboard**:
   In a separate terminal:
   ```bash
   cd dashboard
   npm install
   npm run dev
   ```

4. **Run the Demo**:
   Open the dashboard in your browser (usually `http://localhost:5173`). 
   Scroll down to the new **Live Agent Demo** panel. 
   Select your mode ("Realistic Bug" or "Dangerous Code") and click **Start Agent**.
   The live progression will stream directly into the panel, and the requests will simultaneously appear in the Waterfall view above it.

## Modes

*   **Mode 1 (Realistic Bug):** Prompts the LLM to process an array of objects but intentionally includes a malformed object. This naturally triggers a `TypeError: Cannot read properties of undefined` in the generated code, demonstrating the Apatheia engine catching the error and seamlessly relaying the feedback prompt to Groq for self-healing.
*   **Mode 2 (Dangerous Code):** Prompts the LLM to write a massive, unoptimized loop to calculate prime numbers up to 50,000,000. This demonstrates Apatheia's deterministic fuel metering or wall-clock watchdog safely rejecting the runaway code without crashing the server.


