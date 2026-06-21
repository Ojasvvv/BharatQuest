import asyncio
import aiohttp
import time
import json
import statistics

URL = "https://bharatquest.onrender.com/v1/execute"
HEALTH_URL = "https://bharatquest.onrender.com/health"
API_KEY = "ilovesushmita"

# Mix of fetch() to httpbin.org/delay/4 and simple math
PAYLOADS = [
    """
    let r = fetch("https://httpbin.org/delay/4");
    r.status;
    """,
    "Math.random() * 100;"
]

async def check_health(session, name):
    async with session.get(HEALTH_URL) as response:
        text = await response.text()
        print(f"[{name}] Health Check: {response.status} - {text}")

async def fire_request(session, req_id, payload, memory_limit):
    start = time.time()
    try:
        async with session.post(
            URL,
            headers={"X-API-Key": API_KEY, "Content-Type": "application/json"},
            json={
                "request_id": f"load-{req_id}",
                "language": "javascript",
                "code": payload,
                "timeout_ms": 10000,
                "memory_limit_mb": memory_limit
            }
        ) as response:
            data = await response.json()
            duration = time.time() - start
            return {"id": req_id, "status": data.get("status"), "duration": duration, "error": data.get("message")}
    except Exception as e:
        return {"id": req_id, "status": "failed", "duration": time.time() - start, "error": str(e)}

async def run_subtest(memory_limit):
    print(f"\n--- Running Sub-test with memory_limit_mb={memory_limit} ---")
    
    async with aiohttp.ClientSession() as session:
        # Check health before
        await check_health(session, "Before Test")
        
        start_time = time.time()
        tasks = []
        
        # We fire 5 requests per second for 4 seconds
        for batch in range(4):
            batch_start = time.time()
            for i in range(5):
                req_id = batch * 5 + i
                # Alternate between fetch and non-fetch
                payload = PAYLOADS[req_id % 2]
                tasks.append(asyncio.create_task(fire_request(session, req_id, payload, memory_limit)))
            
            # Sleep until exactly 1 second has passed since batch_start to respect the 5 req/s rate limit
            elapsed = time.time() - batch_start
            if elapsed < 1.0:
                await asyncio.sleep(1.0 - elapsed)
                
        # Wait for all 20 requests to finish
        results = await asyncio.gather(*tasks)
        total_time = time.time() - start_time
        
        # Check health after
        await check_health(session, "After Test")
        
        # Analyze results
        statuses = {}
        durations = []
        fetch_durations = []
        for i, r in enumerate(results):
            s = r["status"]
            statuses[s] = statuses.get(s, 0) + 1
            durations.append(r["duration"])
            if i % 2 == 0:
                fetch_durations.append(r["duration"])
            if s != "success":
                print(f"Request {r['id']} failed with status: {s}, error: {r.get('error')}")
                
        print(f"\nTotal Wall-Clock Time: {total_time:.2f}s")
        print(f"Statuses: {statuses}")
        print(f"Latency Spread (All): Min: {min(durations):.2f}s, Max: {max(durations):.2f}s, Median: {statistics.median(durations):.2f}s")
        print(f"Latency Spread (Fetch only): Min: {min(fetch_durations):.2f}s, Max: {max(fetch_durations):.2f}s, Median: {statistics.median(fetch_durations):.2f}s")

async def main():
    await run_subtest(64)
    print("\nWaiting 10 seconds before Sub-test B...\n")
    await asyncio.sleep(10)
    await run_subtest(256)

if __name__ == "__main__":
    asyncio.run(main())
