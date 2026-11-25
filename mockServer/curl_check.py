import json, sys, urllib.request
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
req = urllib.request.Request("http://127.0.0.1:8000/v1/chat/completions", data=b'{"model":"mock","messages":[{"role":"user","content":"hi"}]}', headers={"Content-Type":"application/json"})
body = json.loads(urllib.request.urlopen(req).read())
msg = body["choices"][0]["message"]
print("message keys:", list(msg.keys()))
print("content:", msg.get("content"))
for k in ("images", "audios", "files"):
    items = msg.get(k)
    print(k, "->", len(items) if items else items)
