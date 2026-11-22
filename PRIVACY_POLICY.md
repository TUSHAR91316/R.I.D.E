# Privacy Policy — R.I.D.E (by Byte-Labs)

**Last Updated:** November 22, 2026  
**Effective Date:** November 22, 2026  
**Developer / Publisher:** Byte-Labs  
**Product:** R.I.D.E (Rust Intelligent Development Environment)  
**Package Family Name (PFN):** `Byte-Labs.R.I.D.E_gnbqk611y9p3y`  
**Store ID:** `9PP3JK8T30DW`  

---

## 1. Introduction

Byte-Labs ("we", "us", or "our") is dedicated to protecting your privacy and ensuring you have complete control over your source code and development environment. This Privacy Policy explains our data practices for **R.I.D.E** (Rust Intelligent Development Environment), a local-first desktop application available for Windows and other desktop platforms.

Our fundamental design principle is **Sovereignty**: your code, keystrokes, configurations, and AI interactions remain strictly on your local device.

---

## 2. Zero Telemetry & No Remote Data Collection

**R.I.D.E collects zero personal data, zero analytics, and zero telemetry.**

Specifically:
- **No Analytics Tracking:** We do not track how you use the editor, which files you open, which shortcuts you press, or how long you use the application.
- **No Remote Crash Reports:** Crash diagnostics and errors are never automatically sent to Byte-Labs or any third-party analytics servers.
- **No Keystroke Logging:** Your typing and text inputs remain in local memory and are never transmitted outside your computer.
- **No Source Code Uploads:** Your workspace files, repositories, git histories, and configurations are processed strictly on your local filesystem and never uploaded to cloud servers.

---

## 3. Local-First AI & Localhost Model Processing

R.I.D.E features integration with on-device Large Language Models (LLMs) and Mixture of Experts (MoE) neural runtimes:

- **Localhost Loopback Only:** Automatic model discovery and inference communication occur exclusively over local network loopback interfaces (e.g., `127.0.0.1` / `localhost` ports: Ollama on `11434`, LM Studio on `1234`, ColibriMoE/Rabbit on `5000`/`8080`, Llama.cpp on `8080`, vLLM/LocalAI on `8000`).
- **No Cloud Relay:** Your prompts, file contexts, and model responses travel solely between R.I.D.E and your locally running model servers. No telemetry or prompt interception is performed.
- **User-Configured Endpoints:** If you explicitly choose to configure a remote or third-party API endpoint (such as an OpenAI-compatible remote server), your queries are sent directly to that configured destination and are subject to that provider's privacy policy. Byte-Labs does not intercept, relay, or store these interactions.

---

## 4. Local Data Storage

All data created or modified by R.I.D.E is stored locally on your device:
- **Workspace Settings & Caches:** Configuration files and skill definitions are stored in `.ride/` within your project or in your user profile directory (`~/.ride/`).
- **Backup Snapshots:** Undo and rollback snapshots created during agent file edits are stored in temporary local workspace caches and can be discarded by the user at any time.
- **No Cloud Sync:** R.I.D.E does not sync your settings or code to cloud accounts.

---

## 5. Network Access & Permissions

When installed via the Microsoft Store or standard desktop packages, R.I.D.E declares the `runFullTrust` desktop bridge capability. This permission is utilized solely for:
- Reading and writing project files on your local hard drive upon user request.
- Executing user-approved developer command-line tools (`cargo`, `rustc`, `git`, build tools) inside the integrated terminal.
- Communicating with local on-device AI runtimes over local loopback sockets (`127.0.0.1`).

R.I.D.E does not open outgoing connections to remote telemetry, tracking, or marketing servers.

---

## 6. Children's Privacy

R.I.D.E is a general developer tool suitable for developers of all ages. Because we do not collect any personal data whatsoever, we do not knowingly solicit or collect data from children under the age of 13 (or under the age of 16 in the European Union) in accordance with COPPA, GDPR, and applicable privacy regulations.

---

## 7. Changes to This Privacy Policy

We may update this Privacy Policy from time to time to reflect new offline features or updates to store policies. When updated, the revised policy will be published in our repository with an updated effective date.

---

## 8. Contact & Support

If you have questions regarding this Privacy Policy or the security architecture of R.I.D.E, please contact us:
- **GitHub Issues:** [https://github.com/TUSHAR91316/R.I.D.E/issues](https://github.com/TUSHAR91316/R.I.D.E/issues)
- **Organization:** Byte-Labs
