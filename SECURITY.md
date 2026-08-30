# Security policy

Please report a vulnerability privately through GitHub's security-advisory
feature when available. Do not include game saves, usernames, full local paths,
or other personal data in a public issue.

The save observer remains local, read-only, bounded to configured directories,
and never rewrites or deletes a save archive. The optional Tesmio research
companion is separately built same-process code, not an operating-system
sandbox. It must fail closed on unsupported executable identities, write only
its own bounded telemetry file, and never write game state, saves, application
databases, or the network. TesmioLoader's normal defaults are outside this
contract: research use also requires the reviewed observation-only settings,
the companion as the sole plugin DLL, and a passing preflight verifier. Report
any violation of that boundary as a security issue.
