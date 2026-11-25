import json, sys, urllib.request
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
info = json.loads(urllib.request.urlopen("http://127.0.0.1:8000/").read())
print(json.dumps(info, ensure_ascii=False, indent=2))
