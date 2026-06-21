import urllib.request
import urllib.error
import json
import time
import threading
import statistics

URL = "https://bharatquest.onrender.com/v1/execute"
HEALTH_URL = "https://bharatquest.onrender.com/health"
API_KEY = "ilovesushmita"

PAYLOADS = [
    """
    let r = fetch("https://httpbin.org/delay/4");
    r.status;
    """,
    "Math.random() * 100;"
]

def check_health(name):
    req = urllib.request.Request(HEALTH_URL)
    try:
        with urllib.request.urlopen(req) as response:
            text = response.read().decode('utf-8')
            print(f"[{name}] Health Check: {response.status} - {text}")
    except Exception as e:
        print(f"[{name}] Health Check Failed: {e}")

def fire_request(req_id, payload, memory_limit, results):
    start = time.time()
    data = json.dumps({
        "request_id": f"load-{req_id}",
        "language": "javascript",
        "code": payload,
        "timeout_ms": 10000,
        "memory_limit_mb": memory_limit
    }).encode('utf-8')
    
    req = urllib.request.Request(URL, data=data, headers={
        "X-API-Key": API_KEY,
        "Content-Type": "application/json"
    })
    
    try:
        with urllib.request.urlopen(req) as response:
            res_data = json.loads(response.read().decode('utf-8'))
            duration = time.time() - start
            results.append({"id": req_id, "status": res_data.get("status"), "duration": duration, "error": res_data.get("message")})
    except Exception as e:
        results.append({"id": req_id, "status": "failed", "duration": time.time() - start, "error": str(e)})

def run_subtest(memory_limit):
    print(f"\n--- Running Sub-test with memory_limit_mb={memory_limit} ---")
    check_health("Before Test")
    
    start_time = time.time()
    threads = []
    results = []
    
    for batch in range(4):
        batch_start = time.time()
        for i in range(5):
            req_id = batch * 5 + i
            payload = PAYLOADS[req_id % 2]
            t = threading.Thread(target=fire_request, args=(req_id, payload, memory_limit, results))
            threads.append(t)
            t.start()
        
        elapsed = time.time() - batch_start
        if elapsed < 1.0:
            time.sleep(1.0 - elapsed)
            
    for t in threads:
        t.join()
        
    total_time = time.time() - start_time
    check_health("After Test")
    
    statuses = {}
    durations = []
    fetch_durations = []
    
    for r in results:
        s = r["status"]
        statuses[s] = statuses.get(s, 0) + 1
        durations.append(r["duration"])
        if r["id"] % 2 == 0:
            fetch_durations.append(r["duration"])
        if s != "success":
            print(f"Request {r['id']} failed with status: {s}, error: {r.get('error')}")
            
    print(f"\nTotal Wall-Clock Time: {total_time:.2f}s")
    print(f"Statuses: {statuses}")
    print(f"Latency Spread (All): Min: {min(durations):.2f}s, Max: {max(durations):.2f}s, Median: {statistics.median(durations):.2f}s")
    print(f"Latency Spread (Fetch only): Min: {min(fetch_durations):.2f}s, Max: {max(fetch_durations):.2f}s, Median: {statistics.median(fetch_durations):.2f}s")

def main():
    run_subtest(64)
    print("\nWaiting 10 seconds before Sub-test B...\n")
    time.sleep(10)
    run_subtest(256)

if __name__ == "__main__":
    main()
