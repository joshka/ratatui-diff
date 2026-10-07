"""Return the published baseline; only a registry 404 means an initial release."""
import json
import urllib.error
import urllib.request

request = urllib.request.Request(
    "https://crates.io/api/v1/crates/ratatui-diff",
    headers={"User-Agent": "ratatui-diff-ci"},
)
try:
    with urllib.request.urlopen(request, timeout=30) as response:
        crate = json.load(response)["crate"]
        print(crate["max_stable_version"])
except urllib.error.HTTPError as error:
    if error.code != 404:
        raise
