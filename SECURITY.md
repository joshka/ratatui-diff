# Security

Report vulnerabilities privately through GitHub's security advisory interface for this repository.
Do not include private source content in public reports. Provide the affected version and a minimal
reproduction using synthetic input.

Patch input is untrusted text. The library never applies patches, runs commands, reads files, or
interprets terminal escape sequences. Comparison and preparation are synchronous and can consume
substantial resources; hosts should bound input size and dispatch expensive preparation outside the
UI loop. Binary data is presented as an opaque summary.
