import json, sys, urllib.request
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
req = urllib.request.Request("http://127.0.0.1:8000/v1/chat/completions", data=b'{"model":"mock","messages":[{"role":"user","content":"hi"}]}', headers={"Content-Type":"application/json"})
body = json.loads(urllib.request.urlopen(req).read())
for item in body["choices"][0]["message"].get("audios", []):
    url = item["audio_url"]["url"]
    print("audio url 前 80 字:", url[:80])
    print("总长度:", len(url))
