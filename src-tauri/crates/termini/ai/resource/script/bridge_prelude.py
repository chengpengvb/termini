import json as _termini_json
import os as _termini_os
import urllib.request as _termini_request


def _termini_tool_call(name, args):
    payload = _termini_json.dumps({"token": _termini_os.environ["TERMINI_TOOL_BRIDGE_TOKEN"], "name": name, "args": args if args is not None else {}}).encode("utf-8")
    request = _termini_request.Request(_termini_os.environ["TERMINI_TOOL_BRIDGE_URL"], data=payload, headers={"content-type": "application/json"}, method="POST")
    with _termini_request.urlopen(request) as response:
        body = _termini_json.loads(response.read().decode("utf-8"))
    if body.get("code") != 200:
        raise RuntimeError(f"tool {name} failed: {body.get('message') or 'unknown error'}")
    return body.get("data")


class _TerminiTools:
    def __getattr__(self, name):
        return lambda args=None: _termini_tool_call(name, args)


tools = _TerminiTools()
